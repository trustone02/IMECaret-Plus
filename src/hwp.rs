use crate::ime::{ImeSnapshot, Validity};
use crate::win::*;
use std::mem::zeroed;
use std::sync::atomic::{AtomicBool, Ordering};

// 아래한글 한/영 상태 관리 플래그 (기본값 한글: true)
static HWP_IS_HANGUL: AtomicBool = AtomicBool::new(true);

/// 한/영 키 입력 시 호출되는 상태 토글 함수 (Raw Input 연동)
pub fn toggle_hwp_hangul_state() {
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
pub unsafe fn find_hwp_win32_caret(window: HWND) -> Option<crate::editability::CaretAnchor> {
    if window.is_null() {
        return None;
    }

    let mut process_id = 0u32;
    let target_tid = GetWindowThreadProcessId(window, &mut process_id);
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
        window
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
