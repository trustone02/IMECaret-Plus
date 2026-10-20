# IMECaret Plus (아이엠이캐럿 플러스) v2.5.1

> **Based on [mattisonit/ime-caret](https://github.com/mattisonit/ime-caret)**  
> **한국어 실무 환경(아래한글, SSMS 전자/반자, 오피스, Windows 11)을 위한 지능형 텍스트 캐럿 IME 상태 표시기**

Windows 환경에서 현재 활성화된 텍스트 커서(Caret) 바로 옆에 실시간 한/영/일 입력 상태를 작고 깔끔하게 표시해 주는 경량 유틸리티 프로그램입니다.

원작 `ime-caret`의 견고한 캐럿 감지 엔진을 기반으로, 한국 실무 환경(세무조사관, 회계사, 개발자, 공공기관)에서 필수적인 **아래한글(HWP) 한/영 추적, SSMS 전자(全角)/반자(半角) 감지, Windows 11 작업표시줄 검색창 호환, 공식 설치 프로그램** 등을 대폭 발전시킨 강화판(Plus Edition)입니다.

---

## 🚀 주요 특징 (Plus 핵심 기능)

* **한컴오피스 한/글(HWP) 한/영 상태 완벽 확인:**
  * 독자적인 자체 입력 엔진(`HncInput`)을 사용하는 아래한글에서도 하드웨어 키보드 Raw Input 훅 및 실제 텍스트 에디터 창(`HwpMainEditWnd`) Win32 시스템 캐럿 직결 추적을 탑재하여, 문서 작성 중 실시간 한/영 입력 상태(`가` / `a`)를 오차 없이 정확하게 표시합니다.
* **SSMS 등 데이터베이스 도구의 전자(全角) / 반자(半角) 상태 확인:**
  * SQL Server Management Studio(SSMS) 등 쿼리 및 코딩 환경에서 쿼리 에러의 주원인이 되는 전자(전각) 공백 및 영문 상태를 사전에 감지하여 전자 상태 시 즉각적인 경고 색상으로 오타와 쿼리 오류를 방지합니다.
* **풍부한 기본 설정 및 맞춤화 지원:**
  * 시스템 트레이 우클릭 메뉴를 통한 원클릭 환경 설정
  * 상태(한/영/일) 전환 알림음(Sound) 개별 재생 설정
  * 상태 표시 배지 위치(캐럿 우측, 우상단, 우하단) 자유 지정
  * 각 언어별 글자색 및 배경색 투명도(Alpha) 커스터마이징 (`RRGGBBAA`)
* **Windows 11 최신 셸 및 작업표시줄 검색창 최적화:**
  * Windows 11 작업표시줄 검색창(`SearchBox`)이나 시스템 플라이아웃 뒤로 배지가 숨거나 튕겨나가지 않도록 안전 내부 안착(Clamping) 및 최상위 레이어(Topmost) 자동 유지를 보장합니다.
* **초경량 & 초고속 (Zero-Latency):**
  * Rust 기반 순수 Win32/UIA 하이브리드 엔진으로 개발되어 CPU 점유율 0% 및 0ms(체감 지연 0초)의 즉각적인 한/영 토글 반응성을 제공합니다.
* **원클릭 공식 설치 프로그램 제공:**
  * Windows 시작 시 자동 실행 등록 지원 및 프로그램 제거(언인스톨) 시 Windows 11 작업표시줄 알림 아이콘 캐시(`NotifyIconSettings`)까지 흔적 없이 100% 완전 청소합니다.

---

## 📌 상태 표시 안내

| 상태 | 표시 배지 | 설명 |
|:---:|:---:|---|
| **한글** | `가` | 한글 입력 모드 (기본 파란색/보라색) |
| **영문 소문자** | `a` | 영문 반자 입력 모드 (Caps Lock 꺼짐, 기본 주황색) |
| **영문 대문자** | `A` | 영문 대문자 고정 모드 (Caps Lock 켜짐) |
| **전자(全角) 경고** | `Ａ` / `가` | SSMS 등에서 전각 상태 감지 시 시각적 구분 및 경고 |
| **일본어 히라가나** | `ひ` | 일본어 IME 히라가나 모드 |
| **일본어 가타카나** | `カ` | 일본어 IME 가타카나 모드 |

---

## ⚙️ 설정 및 맞춤화

작업표시줄 시스템 트레이의 **IMECaret Plus** 아이콘을 우클릭하면 설정 창을 열 수 있습니다.

* **상태 전환 알림음:** 상태(한/영/일) 변경 시 경쾌한 사운드 재생 여부 설정
* **배지 위치 조정:**
  * `Right` (캐럿 우측)
  * `Above` (캐럿 상단)
  * `Below` (캐럿 하단 - 기본값)
* **색상 맞춤화:** 한글, 영문, 일본어 각 상태별 배경색 및 글자색을 투명도(Alpha)를 포함하여 자유롭게 설정 가능 (`RRGGBBAA` 형식)

설정 파일은 실행 파일과 동일한 폴더의 `IMECaretPlus.ini`에 자동 저장되며, 구버전 `IMECaret.ini` 설정도 자동으로 호환 인계됩니다.

---

## 📦 설치 및 실행

1. **설치 파일 다운로드:** [Releases](https://github.com/trustone02/IME-Caret/releases)에서 최신 `IMECaretPlus-Setup.exe`를 다운로드하여 실행합니다.
2. 설치 마법사에서 **"Windows 시작 시 자동으로 실행"**을 체크하면 PC 부팅 시 자동으로 백그라운드 상주 실행됩니다.
3. 무설치 독립 실행 바이너리를 원하실 경우 `dist\IMECaretPlus.exe`를 바로 실행하셔도 됩니다.

---

## 🛠️ 직접 빌드하기

### 요구 사항
* Windows 10 / Windows 11
* Rust Stable (MSVC 툴체인)
* Visual Studio C++ Build Tools
* (선택) Inno Setup 6 (설치 프로그램 생성 시)

```powershell
# 프로젝트 클론 (메인 본진 저장소)
git clone https://github.com/trustone02/IME-Caret.git
cd IME-Caret

# 단위 테스트 실행
cargo test

# 최적화 릴리즈 빌드
cargo build --release

# 설치 프로그램 컴파일 (Inno Setup 필요)
& 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe' installer\installer.iss
```

---

## 📜 라이선스 및 지적재산권 출처 공시 (Acknowledgments & License)

본 프로젝트는 [MIT License](LICENSE)에 따라 자유롭게 사용, 수정, 배포할 수 있습니다.

### Based on
> **Based on [mattisonit/ime-caret](https://github.com/mattisonit/ime-caret)**

본 소프트웨어는 [mattisonit](https://github.com/mattisonit) 님의 오픈소스 프로젝트 **[ime-caret](https://github.com/mattisonit/ime-caret)**에서 영감을 받아 시작되었습니다. 대한민국 실무자(세무조사관, 회계사, 공공기관 및 개발자)의 실제 사용 환경에 맞추어 **한컴오피스 한/글(HWP) 입력기 연동, SSMS 전자/반자 상태 확인, Windows 11 셸 가림 방지, 무잔여 클린 인스톨러** 등을 대폭 발전시킨 강화판(Plus Edition)입니다.

원작자의 뛰어난 아이디어와 훌륭한 오픈소스 기여에 진심으로 감사드립니다.

Copyright (c) 2026 trustone02 & mattisonit. Licensed under the MIT License.
