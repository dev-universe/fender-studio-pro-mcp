# 2026년 11월 25일 인수인계

## 현재 상태

한국 시간 12시 37분의 체크포인트다. Studio Pro 8.1.1 빌드 113103,
시험 MCP owner, REA 세션과 Ghidra 작업을 닫았다. 앱을 다시 열기 전에
새 문서와 owner를 바인딩한다. 기존 지시인 공식 Studio Pro 설치 파일
재대조 생략은 유효하다. 이 세션에 지정된 종료 시각은 없다.

## Serum 2와 Serum 2 FX

- 사용자 DMG에 포함된 Apple 공증 및 Xfer 서명 2.1.5 설치 패키지에서
  추출한 공식 VST3를 사용했다. 임시 경로를 허용한 sandbox로 이 복사본만
  검색했다. 시스템에 있던 별도 VST3의 해시를 대조했고 시험용 복사본을
  스캐너 경로에서 걷어냈다. 원래 설치와 설정을 복원했다.
- 정확한 두 VST3 클래스를 선택하는 Rust `serum2_load`가 [#103](https://github.com/dev-universe/fender-studio-pro-mcp/pull/103)으로
  머지됐다. R316, R318에서 두 인스턴스를 Studio Pro에 추가하고 각각 2,623개
  DSP 파라미터를 열거했다. `A Level`을 변경하고 독립 읽기와 원복을 확인했다.
  R318의 동일 루트 측정은 130요청, 15.391초다.
- 공식 VST3는 과거 Ableton에서 분석한 두 이미지의 해시와 다르다. REA와
  Ghidra로 팩토리·처리·상태·파라미터 진입점을 새로 확인했다. Ghidra의 두
  슬라이스 함수 주소 55,276개에 대해 55,213개 의사코드를 얻었다. 나머지
  63개는 Craft Anything에서 같은 주소의 의사코드를 확보했다. AU와
  AAX 실행 파일 슬라이스는 기존 Ableton 증거와 바이트가 같다. AAX는
  정적 자료만 있고 추출 번들의 엄격 서명 검사는 통과하지 못했다.
  [공식 VST3 분석 #102](https://github.com/dev-universe/fender-studio-pro-mcp/pull/102)
- R316은 Ableton 연구에서 만든 `.SerumPreset` 두 개를 제조사 편집기에서
  열었다. R318은 별개의 `.vstpreset` 두 개를 Studio Pro의 프리셋 파일
  불러오기 메뉴로 적용했다. 악기 15개와 FX 13개 값의 변화를 독립 읽었다.
  에디터에 악기 릴리즈 150 ms와 FX의 Distortion 다음 Equalizer가 표시됐다.
  공식 데모에서 내보낸 파일의 352바이트 `Comp`에는 데모 메타데이터만 있었다.
  전체 패치 저장·재로드와 MCP가 직접 적용하는 수신자는 검증이 남았다.
  [Studio Pro VST3 프리셋 증거 #107](https://github.com/dev-universe/fender-studio-pro-mcp/pull/107)

## Pedalboard

- 기존 인접 교체, 저장·재시작, Undo가 각각 [#97](https://github.com/dev-universe/fender-studio-pro-mcp/pull/97),
  [#98](https://github.com/dev-universe/fender-studio-pro-mcp/pull/98),
  [#99](https://github.com/dev-universe/fender-studio-pro-mcp/pull/99)로 머지됐다.
- R317은 빈 2번 슬롯으로 Compressor를 옮기고 역방향에서 화면상 원복을
  확인했다. 빈 슬롯의 `controls.active[stomp2]` raw 10000이 남아 정확한
  값 복구는 실패했다. [빈 슬롯 #104](https://github.com/dev-universe/fender-studio-pro-mcp/pull/104)
- R314는 동일 Compressor 두 개의 노브와 표시등 교체를 확인했다. 독립
  DSP 읽기에서는 `stompN.gain`과 `stompN.active`가 기존 위치의 값을
  반환했다. 편집기를 닫고 다시 열어도 같은 결과였다.
  [동일 모델 #105](https://github.com/dev-universe/fender-studio-pro-mcp/pull/105)
- R315에서 1·3번 직접 교환과 두 인접 교환을 통한 순차 이동을 구분해
  검증했다. 두 경로 모두 역으로 돌리자 비미터 42개 값과 컴포넌트
  지문이 돌아왔다. 구분선 횡단은 별도다.
  [비인접 #106](https://github.com/dev-universe/fender-studio-pro-mcp/pull/106)
- 각 소유 시험곡을 저장 없이 닫았다. 18개 정상 MCP 파일과 테스트 전
  Studio Pro 환경설정 및 캐시를 원복했다.

## 병합과 다음 작업

원격 `main`은 `d6e254e`다. 스택은 [#90](https://github.com/dev-universe/fender-studio-pro-mcp/pull/90)
→ #91 → #92 → #93 → #94 → #95 순서다. #90 헤드 `a3082f3b`가 현재
`main`을 반영했다. 두 필수 검사를 확인한 뒤 정확한 헤드로 머지한다.
#92, #93의 기존 macOS 검사는 취소됐으며 새로운 헤드의 결과를 본다.
별도의 공개 증거 PR #104, #105, #106, #107도 각각 최신
`main`을 포함한 정확한 헤드에서 두 검사를 통과하면 머지한다.
#107의 기존 macOS 실패 원인은 crates.io DNS 조회 실패였다.

11월 24일의 내장 43개 중 잔여 25개와 하드웨어 의존 4개는 고정
집계에서 그대로다. 이번에 완료한 Pedalboard 관측과 배포 결과를
재사용한다. Serum의 다음 제품 단위는 표준 VST3 파일을 정확한
Studio Pro 인스턴스에 적용하는 MCP 경로다. 스튜디오의 VST3 프리셋
파일 처리기 등록과 메뉴의 성공 근거가 있다. 전체 편집 상태의
저장·재로드에는 공식 라이선스 활성화가 확인된 실행이 필요하다.
