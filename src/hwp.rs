use crate::ime::{ImeSnapshot, Validity};
use crate::win::*;
use std::mem::zeroed;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

// 아래한글 한/영 상태 관리 플래그 (기본값 한글: true - 사용자가 직접 실행한 아래한글은 한글 입력 모드로
// 시작하는 것을 실측 확인)
static HWP_IS_HANGUL: AtomicBool = AtomicBool::new(true);

// 한/영 키 연타 및 0x15/0xa5 중복 토글 방지용 디바운스 타이머 (150ms)
static LAST_TOGGLE_TICK: Mutex<Option<Instant>> = Mutex::new(None);

// 사용자가 키보드로 직접 한/영 키를 입력한 시각 (시스템 인디케이터 조기 덮어쓰기 방지용 유예 타이머: 350ms)
static LAST_USER_KEY_TICK: Mutex<Option<Instant>> = Mutex::new(None);

// IMM(ImmGetDefaultIMEWnd) 조회값이 실제 한/영 키에 반응하는지 자가 판별한 결과.
// 반응이 확인되면 키 토글 추측 대신 IMM 값을 유일한 기준으로 삼는다.
static IMM_TRACKS: AtomicBool = AtomicBool::new(false);

// 한/영 키 직후 IMM 값 변화를 비교하기 위한 (키 입력 시각, 키 입력 전 IMM 값)
static PENDING_CALIBRATION: Mutex<Option<(Instant, Option<bool>)>> = Mutex::new(None);

const IMM_CALIBRATION_DELAY: Duration = Duration::from_millis(150);

/// %LOCALAPPDATA%\IMECaretPlus\imecaret.log 에 한 줄 추가 (256KB 초과 시 초기화)
pub fn log_line(message: &str) {
    use std::io::Write;
    let Some(base) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let dir = std::path::PathBuf::from(base).join("IMECaretPlus");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("imecaret.log");
    if std::fs::metadata(&path).map_or(false, |m| m.len() > 256 * 1024) {
        let _ = std::fs::remove_file(&path);
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{}] {}", secs, message);
    }
}

// 상태를 추적 중인 아래한글 프로세스 ID. 새 프로세스(=새로 시작한 아래한글)는 한글 상태로 시작한다.
static HWP_PID: AtomicU32 = AtomicU32::new(0);

fn state_file_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(std::path::PathBuf::from(base).join("IMECaretPlus").join("hwp_state.txt"))
}

/// "<pid> <0|1>" 형식으로 저장된 상태를 해당 프로세스에 한해 복원
fn load_saved_state(pid: u32) -> Option<bool> {
    let text = std::fs::read_to_string(state_file_path()?).ok()?;
    let mut parts = text.split_whitespace();
    let saved_pid: u32 = parts.next()?.parse().ok()?;
    let flag = parts.next()?;
    (saved_pid == pid).then_some(flag == "1")
}

fn save_state(pid: u32, hangul: bool) {
    if let Some(path) = state_file_path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, format!("{} {}", pid, hangul as u8));
    }
}

/// 아래한글 창의 프로세스가 바뀌었으면(앱 재시작 또는 아래한글 재실행) 상태를 복원/초기화한다.
unsafe fn adopt_hwp_process(window: HWND) {
    if window.is_null() || !is_hwp_window(window) {
        return;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(window, &mut pid);
    if pid == 0 || HWP_PID.swap(pid, Ordering::SeqCst) == pid {
        return;
    }
    let restored = load_saved_state(pid);
    HWP_IS_HANGUL.store(restored.unwrap_or(true), Ordering::SeqCst);
    log_line(&format!(
        "[HWP] 아래한글 프로세스 {} 감지 -> 상태 {}",
        pid,
        match restored {
            Some(v) => format!("복원 flag={}", v),
            None => "새 프로세스, 한글로 초기화".to_string(),
        }
    ));
}

// ---- 캐럿 정지 기반 한글 상태 보정 (한글 쪽으로만) ----
// 아래한글의 한/영 모드는 외부에서 읽을 수 없고 시작 모드도 일정하지 않다. 글자 키를 눌렀는데 캐럿이
// 전혀 움직이지 않는 것은 한글 조합 중에서만 나타나는 현상(영문은 항상 즉시 이동)이므로, 이 경우에만
// "한글"로 보정한다. 반대로 캐럿이 움직인다는 것은 영문의 증거가 아니다(한글 입력 중에도 연속해서
// 움직임을 실측 확인). 따라서 영문으로 바꾸는 추측은 하지 않고, 영문 전환은 한/영 키로만 이루어진다.
pub const LETTER_PROBE_DELAY_MS: u32 = 80;
const LETTER_PROBE_STALE: Duration = Duration::from_millis(1500);
const MIN_ADVANCE_PX: i32 = 3;

struct LetterProbe {
    settled: Option<(Instant, i32, i32)>,
    before: Option<(i32, i32)>,
    pending: bool,
}

static LETTER_PROBE: Mutex<LetterProbe> = Mutex::new(LetterProbe {
    settled: None,
    before: None,
    pending: false,
});

fn lock_probe() -> std::sync::MutexGuard<'static, LetterProbe> {
    match LETTER_PROBE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn reset_letter_probe() {
    let mut probe = lock_probe();
    probe.settled = None;
    probe.before = None;
    probe.pending = false;
}

#[derive(Debug, PartialEq, Eq)]
enum CaretEvidence {
    /// 캐럿이 즉시 오른쪽으로 이동 (영문 입력, 또는 한글 글자 확정)
    Advanced,
    /// 캐럿이 그대로 (한글 조합 중)
    Stationary,
    /// 줄 바꿈이나 뒤로 이동 등 판단 불가
    Unknown,
}

fn caret_evidence(before: (i32, i32), after: (i32, i32)) -> CaretEvidence {
    if after.1 != before.1 {
        CaretEvidence::Unknown
    } else if after.0 - before.0 >= MIN_ADVANCE_PX {
        CaretEvidence::Advanced
    } else if after.0 == before.0 {
        CaretEvidence::Stationary
    } else {
        CaretEvidence::Unknown
    }
}

/// 아래한글 UI 스레드의 시스템 캐럿 (left, top). 캐럿이 없으면 None.
unsafe fn hwp_caret_pos() -> Option<(i32, i32)> {
    let fg = GetForegroundWindow();
    if fg.is_null() {
        return None;
    }
    let tid = GetWindowThreadProcessId(fg, null_mut());
    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    if tid == 0 || GetGUIThreadInfo(tid, &mut info) == FALSE || info.hwndCaret.is_null() {
        return None;
    }
    let rc = info.rcCaret;
    if rc.left == 0 && rc.right == 0 && rc.top == 0 && rc.bottom == 0 {
        return None;
    }
    Some((rc.left, rc.top))
}

unsafe fn infer_hangul(hangul: bool, reason: &str) -> bool {
    let fg = GetForegroundWindow();
    adopt_hwp_process(fg);
    let prev = HWP_IS_HANGUL.swap(hangul, Ordering::SeqCst);
    if prev == hangul {
        return false;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(fg, &mut pid);
    if pid != 0 {
        save_state(pid, hangul);
    }
    log_line(&format!("[INFER] {} -> 상태 교정 {} -> {}", reason, prev, hangul));
    true
}

/// 아래한글에서 수식키 없이 글자 키(A-Z)를 눌렀을 때 호출. 판별용 타이머를 예약해야 하면 true.
pub fn on_hwp_letter_key() -> bool {
    unsafe {
        // Ctrl/Alt/Win 조합은 입력이 아니라 단축키이므로 제외
        for vk in [0x11, 0x12, 0x5B, 0x5C] {
            if GetAsyncKeyState(vk) as u16 & 0x8000 != 0 {
                return false;
            }
        }
    }
    let mut probe = lock_probe();
    if probe.pending {
        return false;
    }
    probe.pending = true;
    probe.before = match probe.settled {
        Some((tick, x, y)) if tick.elapsed() <= LETTER_PROBE_STALE => Some((x, y)),
        _ => None,
    };
    true
}

/// 판별 타이머 만료 시 호출. 상태가 바뀌었으면 true.
pub fn evaluate_letter_probe() -> bool {
    let (before, was_pending) = {
        let mut probe = lock_probe();
        let was = probe.pending;
        probe.pending = false;
        (probe.before.take(), was)
    };
    if !was_pending {
        return false;
    }

    let after = unsafe { hwp_caret_pos() };
    let Some((ax, ay)) = after else {
        reset_letter_probe();
        return false;
    };

    let mut changed = false;
    if let Some(before) = before {
        match caret_evidence(before, (ax, ay)) {
            CaretEvidence::Advanced | CaretEvidence::Unknown => {}
            CaretEvidence::Stationary => {
                changed = unsafe { infer_hangul(true, "글자 키 입력에도 캐럿 정지(한글 조합 중)") };
            }
        }
    }
    lock_probe().settled = Some((Instant::now(), ax, ay));
    changed
}

/// IMM 기준 한/영 상태 조회. 조회 불가 시 None.
unsafe fn read_imm_hangul(window: HWND) -> Option<bool> {
    if window.is_null() {
        return None;
    }
    let ime_window = ImmGetDefaultIMEWnd(window);
    if ime_window.is_null() {
        return None;
    }
    let mut mode = 0usize;
    let ok = SendMessageTimeoutW(
        ime_window,
        WM_IME_CONTROL,
        IMC_GETCONVERSIONMODE,
        0,
        SMTO_BLOCK | SMTO_ABORTIFHUNG,
        50,
        &mut mode,
    );
    (ok != 0).then_some(mode & 1 != 0)
}

/// 포그라운드 아래한글 UI 스레드의 실제 포커스 창 (편집 창)
unsafe fn hwp_focus_window() -> HWND {
    let fg = GetForegroundWindow();
    if fg.is_null() {
        return null_mut();
    }
    let tid = GetWindowThreadProcessId(fg, null_mut());
    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    if tid != 0 && GetGUIThreadInfo(tid, &mut info) != FALSE && !info.hwndFocus.is_null() {
        info.hwndFocus
    } else {
        fg
    }
}

/// 한/영 키 입력 시 호출되는 상태 토글 함수 (Raw Input 연동)
pub fn toggle_hwp_hangul_state() {
    let now = Instant::now();
    let mut last = match LAST_TOGGLE_TICK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(t) = *last {
        if now.duration_since(t) < Duration::from_millis(150) {
            return;
        }
    }
    *last = Some(now);

    if let Ok(mut key_tick) = LAST_USER_KEY_TICK.lock() {
        *key_tick = Some(now);
    }

    // IMM 신뢰도 판별은 아래한글이 포그라운드일 때의 키 입력으로만 수행한다.
    unsafe {
        if is_hwp_window(GetForegroundWindow()) {
            let before = read_imm_hangul(hwp_focus_window());
            if let Ok(mut pending) = PENDING_CALIBRATION.lock() {
                *pending = Some((now, before));
            }
        }
    }

    unsafe {
        adopt_hwp_process(GetForegroundWindow());
    }
    reset_letter_probe();
    let _prev = HWP_IS_HANGUL.fetch_xor(true, Ordering::SeqCst);
    unsafe {
        let fg = GetForegroundWindow();
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, &mut pid);
        if is_hwp_window(fg) {
            save_state(pid, !_prev);
        }
        log_line(&format!(
            "[KEY] 한/영 키 fg={} hwp={} flag {} -> {}",
            crate::ime::process_executable_name(pid).unwrap_or_default(),
            is_hwp_window(fg),
            _prev,
            !_prev
        ));
    }
    #[cfg(debug_assertions)]
    println!("[HWP] 한/영 토글: {} -> {}", _prev, !_prev);
}

/// OS 작업표시줄 UIA 시스템 인디케이터로부터 한/영 상태를 동기화
/// (단, 사용자가 직접 한/영 키를 누른 직후 350ms 동안은 OS 셸의 지연된 상태로 덮어쓰지 않음)
pub fn set_hwp_hangul_state_from_system(is_hangul: bool) {
    if IMM_TRACKS.load(Ordering::Relaxed) {
        return;
    }

    if let Ok(guard) = LAST_USER_KEY_TICK.lock() {
        if let Some(t) = *guard {
            if Instant::now().duration_since(t) < Duration::from_millis(350) {
                return;
            }
        }
    }
    let prev = HWP_IS_HANGUL.swap(is_hangul, Ordering::SeqCst);
    if prev != is_hangul {
        log_line(&format!("[SYNC] 시스템/타 프로그램 상태 반영 flag {} -> {}", prev, is_hangul));
    }
}

/// 강제 상태 설정 (창 포커스 진입 시 즉각 동기화)
pub fn force_set_hwp_hangul_state(is_hangul: bool) {
    if IMM_TRACKS.load(Ordering::Relaxed) {
        return;
    }
    HWP_IS_HANGUL.store(is_hangul, Ordering::SeqCst);
}

/// 현재 한/영 상태 설정
#[allow(dead_code)]
pub fn set_hwp_hangul_state(is_hangul: bool) {
    set_hwp_hangul_state_from_system(is_hangul);
}

/// 현재 한/영 상태 조회
#[allow(dead_code)]
pub fn is_hwp_hangul() -> bool {
    HWP_IS_HANGUL.load(Ordering::Relaxed)
}

// 프로세스 ID별 "아래한글 프로세스 여부" 판정 캐시. 프로세스 이름 조회가 일시적으로 실패해도
// 이전에 확인된 결과를 사용하여 판정이 흔들리지 않게 한다.
static HWP_PROCESS_CACHE: Mutex<Option<std::collections::HashMap<u32, bool>>> = Mutex::new(None);

fn is_hwp_process(process_id: u32) -> bool {
    let name = unsafe { crate::ime::process_executable_name(process_id) };
    let mut guard = match HWP_PROCESS_CACHE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let cache = guard.get_or_insert_with(std::collections::HashMap::new);
    match name {
        Some(name) => {
            let lower = name.to_lowercase();
            let is_hwp = lower == "hwp.exe" || lower.contains("hwp");
            if cache.len() > 256 {
                cache.clear();
            }
            cache.insert(process_id, is_hwp);
            is_hwp
        }
        None => cache.get(&process_id).copied().unwrap_or(false),
    }
}

/// 주어진 윈도우가 아래한글(HWP) 관련 윈도우인지 검사
/// 창 제목에 "한글" 등이 들어간 다른 프로그램을 오인하지 않도록 프로세스 이름으로만 판정한다.
pub fn is_hwp_window(window: HWND) -> bool {
    if window.is_null() || unsafe { IsWindow(window) } == FALSE {
        return false;
    }

    let mut process_id = 0u32;
    let thread_id = unsafe { GetWindowThreadProcessId(window, &mut process_id) };
    thread_id != 0 && process_id != 0 && is_hwp_process(process_id)
}

/// 한/영 키 직후 IMM 값이 바뀌었는지 비교하여 IMM 신뢰 여부를 판정
unsafe fn calibrate_imm(window: HWND) {
    let due = match PENDING_CALIBRATION.lock() {
        Ok(mut pending) => match *pending {
            Some((tick, before)) if tick.elapsed() >= IMM_CALIBRATION_DELAY => {
                *pending = None;
                Some(before)
            }
            _ => None,
        },
        Err(_) => None,
    };
    let Some(before) = due else {
        return;
    };
    let after = read_imm_hangul(window);
    let tracks = matches!((before, after), (Some(b), Some(a)) if a != b);
    if tracks && !IMM_TRACKS.swap(true, Ordering::SeqCst) {
        log_line("[HWP] IMM 값이 한/영 키에 반응함 -> IMM을 기준 상태로 사용");
    }
    log_line(&format!(
        "[HWP] 한/영 키: imm_before={:?} imm_after={:?} flag={} imm_tracks={}",
        before,
        after,
        HWP_IS_HANGUL.load(Ordering::Relaxed),
        IMM_TRACKS.load(Ordering::Relaxed)
    ));
}

/// 아래한글 활성 시 하이브리드 IME 스냅샷 조회 함수
pub fn query_hwp_ime_snapshot(main_window: HWND) -> ImeSnapshot {
    unsafe {
        adopt_hwp_process(main_window);
        calibrate_imm(main_window);
        if IMM_TRACKS.load(Ordering::Relaxed) {
            if let Some(hangul) = read_imm_hangul(main_window) {
                HWP_IS_HANGUL.store(hangul, Ordering::SeqCst);
            }
        }
    }
    let is_hangul = HWP_IS_HANGUL.load(Ordering::Relaxed);

    ImeSnapshot {
        validity: Validity::Live,
        conversion_mode: if is_hangul { 1 } else { 0 }, // 1: IME_CMODE_HANGUL, 0: 영문
        language_id: 0x12, // LANG_KOREAN
        is_open: is_hangul,
    }
}

/// 아래한글 프로세스의 UI 스레드에 입력 큐를 임시 연결(AttachThreadInput)하여
/// 실제 Win32 시스템 캐럿 좌표 탐색 시도
unsafe extern "system" fn find_hwp_edit_wnd_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut class_buf = [0u16; 64];
    let len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32);
    if len > 0 {
        let class = String::from_utf16_lossy(&class_buf[..len as usize]);
        if class.eq_ignore_ascii_case("HwpMainEditWnd") {
            *(lparam as *mut HWND) = hwnd;
            return FALSE;
        }
    }
    TRUE
}

pub unsafe fn find_hwp_main_edit_window(parent: HWND) -> Option<HWND> {
    let mut found: HWND = null_mut();
    EnumChildWindows(parent, Some(find_hwp_edit_wnd_callback), &mut found as *mut HWND as LPARAM);
    if !found.is_null() {
        Some(found)
    } else {
        None
    }
}

pub unsafe fn find_hwp_win32_caret(window: HWND) -> Option<crate::editability::CaretAnchor> {
    if window.is_null() {
        return None;
    }

    // 아래한글 최상위 창(HwndWrapper) 대신 실제 문서 에디터 창인 HwpMainEditWnd 탐색
    let edit_window = find_hwp_main_edit_window(window).unwrap_or(window);

    let mut process_id = 0u32;
    let target_tid = GetWindowThreadProcessId(edit_window, &mut process_id);
    if target_tid == 0 {
        return None;
    }

    let current_tid = GetCurrentThreadId();
    let attached = if current_tid != target_tid {
        AttachThreadInput(current_tid, target_tid, TRUE) != FALSE
    } else {
        false
    };

    let mut result = None;

    let focus_hwnd = GetFocus();
    let target_focus = if !focus_hwnd.is_null() {
        focus_hwnd
    } else {
        edit_window
    };

    let mut pt: POINT = zeroed();
    if GetCaretPos(&mut pt) != FALSE {
        // 캐럿 좌표가 (0,0)이 아니거나 포커스 창이 유효한 경우 스크린 좌표로 변환
        if pt.x > 0 || pt.y > 0 {
            let mut screen_pt = pt;
            if ClientToScreen(target_focus, &mut screen_pt) != FALSE {
                result = Some(crate::editability::CaretAnchor {
                    x: screen_pt.x + 2,
                    top: screen_pt.y,
                    bottom: screen_pt.y + 18,
                });
            }
        }
    }

    if attached {
        AttachThreadInput(current_tid, target_tid, FALSE);
    }

    result
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hwp_hangul_state_toggle_and_grace_period_protection() {
        force_set_hwp_hangul_state(true);
        assert!(is_hwp_hangul());

        // 키 토글 실행: 한글 -> 영문
        toggle_hwp_hangul_state();
        assert!(!is_hwp_hangul());

        // 키 입력 직후(350ms 이내)에 시스템 UIA가 구 상태(한글: true)로 덮어쓰려 할 때 무시(보호)
        set_hwp_hangul_state_from_system(true);
        assert!(!is_hwp_hangul(), "키 입력 직후에는 시스템의 지연된 상태로 덮어쓰지 않아야 함");

        // force_set은 포커스 진입 등 긴급 상황에서 즉각 반영
        force_set_hwp_hangul_state(true);
        assert!(is_hwp_hangul());
    }

    #[test]
    fn caret_evidence_matches_measured_english_and_hangul_behavior() {
        // 영문 실측: d +18, i +9
        assert_eq!(caret_evidence((282, 403), (300, 403)), CaretEvidence::Advanced);
        assert_eq!(caret_evidence((355, 403), (364, 403)), CaretEvidence::Advanced);
        // 한글 조합 중 실측: 캐럿 정지
        assert_eq!(caret_evidence((379, 547), (379, 547)), CaretEvidence::Stationary);
        // 한글 확정 시 +29
        assert_eq!(caret_evidence((379, 547), (408, 547)), CaretEvidence::Advanced);
        // 줄 바꿈 / 뒤로 이동은 판단 불가
        assert_eq!(caret_evidence((900, 403), (282, 451)), CaretEvidence::Unknown);
        assert_eq!(caret_evidence((400, 403), (300, 403)), CaretEvidence::Unknown);
    }
}
