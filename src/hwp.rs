use crate::ime::{ImeSnapshot, Validity};
use crate::win::*;
use std::mem::zeroed;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

// 아래한글 한/영 상태 관리 플래그 (기본값 한글: true)
static HWP_IS_HANGUL: AtomicBool = AtomicBool::new(true);

// 한/영 키 연타 및 0x15/0xa5 중복 토글 방지용 디바운스 타이머 (150ms)
static LAST_TOGGLE_TICK: Mutex<Option<Instant>> = Mutex::new(None);

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

    let _prev = HWP_IS_HANGUL.fetch_xor(true, Ordering::SeqCst);
    #[cfg(debug_assertions)]
    println!("[HWP] 한/영 토글: {} -> {}", _prev, !_prev);
}

/// 현재 한/영 상태 설정
#[allow(dead_code)]
pub fn set_hwp_hangul_state(is_hangul: bool) {
    HWP_IS_HANGUL.store(is_hangul, Ordering::SeqCst);
}

/// 현재 한/영 상태 조회
#[allow(dead_code)]
pub fn is_hwp_hangul() -> bool {
    HWP_IS_HANGUL.load(Ordering::Relaxed)
}

/// 주어진 윈도우가 아래한글(HWP) 관련 윈도우인지 검사
pub fn is_hwp_window(window: HWND) -> bool {
    if window.is_null() || unsafe { IsWindow(window) } == FALSE {
        return false;
    }

    unsafe {
        let mut process_id = 0u32;
        if GetWindowThreadProcessId(window, &mut process_id) != 0 && process_id != 0 {
            if let Some(name) = crate::ime::process_executable_name(process_id) {
                let lower = name.to_lowercase();
                if lower == "hwp.exe" || lower.contains("hwp") {
                    return true;
                }
            }
        }

        // 최상위 창 핸들 구하기
        let root = GetAncestor(window, GA_ROOT);
        for w in [window, root] {
            if w.is_null() {
                continue;
            }
            let mut title_buf = [0u16; 256];
            let len = GetWindowTextW(w, title_buf.as_mut_ptr(), title_buf.len() as i32);
            if len > 0 {
                let title = String::from_utf16_lossy(&title_buf[..len as usize]).to_lowercase();
                if title.contains("한글") || title.contains("hwp") {
                    return true;
                }
            }
        }
    }

    false
}

/// 아래한글 활성 시 하이브리드 IME 스냅샷 조회 함수
pub fn query_hwp_ime_snapshot(_main_window: HWND) -> ImeSnapshot {
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
