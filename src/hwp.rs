use crate::ime::{ImeSnapshot, Validity};
use crate::win::*;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

// 아래한글 한/영 상태 관리 플래그 (기본값 한글: true)
static HWP_IS_HANGUL: AtomicBool = AtomicBool::new(true);

// 마지막 상태표시줄 동기화 시점 관리 (200ms 스로틀링)
static LAST_SYNC_TICK: Mutex<Option<Instant>> = Mutex::new(None);

/// 한/영 키 입력 시 호출되는 상태 토글 함수 (Raw Input 연동)
pub fn toggle_hwp_hangul_state() {
    HWP_IS_HANGUL.fetch_xor(true, Ordering::SeqCst);
}

/// 현재 한/영 상태 설정
pub fn set_hwp_hangul_state(is_hangul: bool) {
    HWP_IS_HANGUL.store(is_hangul, Ordering::SeqCst);
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
                if name.eq_ignore_ascii_case("hwp.exe") || name.to_lowercase().contains("hwp") {
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

// 상태표시줄 탐색 구조체
struct StatusBarFinder {
    found_bars: Vec<HWND>,
}

unsafe extern "system" fn enum_child_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let finder = &mut *(lparam as *mut StatusBarFinder);
    let mut class_buf = [0u16; 128];
    let len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32);
    if len > 0 {
        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]).to_lowercase();
        if class_name.contains("status") || class_name.contains("hnc") {
            finder.found_bars.push(hwnd);
        }
    }
    TRUE
}

/// 아래한글 하단 상태표시줄(Status Bar)을 스크래핑하여 실제 한/영 상태 동기화
pub fn sync_hwp_status_bar(main_window: HWND) -> Option<bool> {
    if main_window.is_null() {
        return None;
    }

    unsafe {
        let root = GetAncestor(main_window, GA_ROOT);
        let target = if !root.is_null() { root } else { main_window };

        let mut finder = StatusBarFinder {
            found_bars: Vec::with_capacity(8),
        };
        EnumChildWindows(
            target,
            Some(enum_child_proc),
            &mut finder as *mut _ as LPARAM,
        );

        for status_hwnd in finder.found_bars {
            // 1. 표준 Win32 StatusBar 메시지 SB_GETPARTS / SB_GETTEXTW 시도
            let mut parts = [0i32; 32];
            let num_parts = SendMessageW(
                status_hwnd,
                SB_GETPARTS,
                parts.len(),
                parts.as_mut_ptr() as LPARAM,
            ) as usize;

            let check_count = if num_parts > 0 && num_parts <= 32 {
                num_parts
            } else {
                8
            };

            for part_idx in 0..check_count {
                let mut text_buf = [0u16; 128];
                let len = SendMessageW(
                    status_hwnd,
                    SB_GETTEXTW,
                    part_idx,
                    text_buf.as_mut_ptr() as LPARAM,
                ) as usize;

                if len > 0 && len < text_buf.len() {
                    let text = String::from_utf16_lossy(&text_buf[..len]);
                    let trimmed = text.trim();
                    if trimmed == "한" || trimmed == "[한]" || trimmed == "한글" {
                        HWP_IS_HANGUL.store(true, Ordering::SeqCst);
                        return Some(true);
                    } else if trimmed == "영" || trimmed == "[영]" || trimmed == "영문" {
                        HWP_IS_HANGUL.store(false, Ordering::SeqCst);
                        return Some(false);
                    }
                }
            }

            // 2. 컨트롤 자체 윈도우 텍스트 검사
            let mut text_buf = [0u16; 256];
            let len = GetWindowTextW(status_hwnd, text_buf.as_mut_ptr(), text_buf.len() as i32);
            if len > 0 {
                let text = String::from_utf16_lossy(&text_buf[..len as usize]);
                if text.contains("한") && !text.contains("영") {
                    HWP_IS_HANGUL.store(true, Ordering::SeqCst);
                    return Some(true);
                } else if text.contains("영") && !text.contains("한") {
                    HWP_IS_HANGUL.store(false, Ordering::SeqCst);
                    return Some(false);
                }
            }
        }
    }

    None
}

/// 아래한글 활성 시 하이브리드 IME 스냅샷 조회 함수
pub fn query_hwp_ime_snapshot(main_window: HWND) -> ImeSnapshot {
    // 200ms 주기로 상태표시줄을 통해 실시간 동기화 보정
    let now = Instant::now();
    let should_sync = {
        let mut last_lock = LAST_SYNC_TICK.lock().unwrap();
        match *last_lock {
            Some(last) if now.duration_since(last) < Duration::from_millis(200) => false,
            _ => {
                *last_lock = Some(now);
                true
            }
        }
    };

    if should_sync {
        sync_hwp_status_bar(main_window);
    }

    let is_hangul = HWP_IS_HANGUL.load(Ordering::Relaxed);

    ImeSnapshot {
        validity: Validity::Live,
        conversion_mode: if is_hangul { 1 } else { 0 }, // 1: IME_CMODE_HANGUL, 0: 영문
        language_id: 0x12, // LANG_KOREAN
        is_open: is_hangul,
    }
}
