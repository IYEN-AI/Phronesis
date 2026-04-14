# Phronesis

**아리스토텔레스의 지행합일(知行合一)에 기반한 에이전틱 메모리 시스템**

> "안다는 것은 행할 수 있다" — 지식을 수행 가능한 단위(Executable Unit)로 저장하는 파일시스템 기반 MCP 서버

Phronesis는 AI 에이전트가 **스스로의 기억과 성장을 관리**하는 자율적 메모리 인프라입니다. 각 에이전트가 독립적으로 소유하며 (1 에이전트 = 1 Phronesis 인스턴스), 경험을 파일시스템에 행동 단위로 기록하고 하이브리드 탐색으로 재활용합니다.

## 철학적 기반

| 요소 | 역할 | 아리스토텔레스 비유 |
|------|------|---------------------|
| 파일시스템 | 사고와 행동 지침의 골격 | 이성 (Logos) |
| 임베딩 탐색 | 상황과 의도의 유연한 연결 | 직관 (Nous) |
| Grep 탐색 | 구체적 행동의 정밀 선택 | 실천적 지혜 (Phronesis) |
| 심볼릭 링크 | 반복된 행동의 자동화 | 습관 (Ethos) |
| Append-only | 경험의 불가역성 | 시간의 화살 |
| 성찰 도구 | 자기 검토와 구조 개선 | 성찰 (Theoria) |

## 핵심 설계

### 6-Pillar 분류 체계

```
/phronesis-root/
├── skills.md          # 사용 가이드 (불변 시드 + 에이전트 학습)
├── self/              # 자기 유지, 정체성
├── perception/        # 상황 인지, 데이터 해석
├── cognition/         # 내부 추론, 지식 처리
├── praxis/            # 외부 실천, 액션
├── evolution/         # 새로운 능력 획득, 구조 확장
└── reflection/        # 성찰, 기존 지식 재구성
    └── warnings/      # 네이밍 규칙 위반 경고 로그
```

### 하이브리드 탐색

```
에이전트 의도
    ↓
embed_search("나는 지금 어떤 상황인가?")
    → 폴더 수준 시맨틱 앵커링
    ↓
grep_search("무엇을 해야 하는가?")
    → 파일명 + 내용 정밀 매칭
    ↓
read_action() 또는 write_action()
```

### Append-Only 원칙

인간의 경험이 불가역적이듯, 에이전트의 기억도 삭제할 수 없습니다. 모든 데이터는 JSONL 형식으로 append만 가능합니다.

```jsonl
{"ts":"2026-04-14T10:00:00Z","situation":"사용자가 불만 표시","reasoning":"감정 인정이 선행되어야","action":"공감 표현 후 해결책 제시","outcome":"만족도 회복"}
{"ts":"2026-04-15T14:30:00Z","situation":"같은 상황 재발","reasoning":"이전 경험 참조","action":"즉시 공감+해결 패턴 적용","outcome":"더 빠르게 해결"}
```

## 아키텍처

```
┌─────────────────────────────────────────────┐
│              MCP Protocol (stdio)            │
├─────────────────────────────────────────────┤
│          Phronesis MCP Server (Rust)         │
│                                              │
│  ┌────────────┐ ┌────────────┐ ┌──────────┐ │
│  │ Embedding  │ │   Grep     │ │   CRUD   │ │
│  │ (OpenAI)   │ │ (name+     │ │ (append  │ │
│  │            │ │  content)  │ │  only)   │ │
│  └─────┬──────┘ └─────┬──────┘ └────┬─────┘ │
│        └───────────────┼─────────────┘       │
│              ┌─────────┴─────────┐           │
│              │  6-Pillar FS      │           │
│              │  (.jsonl files)   │           │
│              └───────────────────┘           │
│  ┌────────────┐ ┌──────────────────┐         │
│  │ HNSW Index │ │ Warning Log      │         │
│  │ (in-memory)│ │ (/reflection)    │         │
│  └────────────┘ └──────────────────┘         │
├─────────────────────────────────────────────┤
│  Abstraction Layers:                         │
│  - EmbeddingProvider trait (MVP: OpenAI)     │
│  - VectorStore trait (MVP: instant-distance) │
└─────────────────────────────────────────────┘
```

## MCP 도구 (10개)

| 도구 | 설명 |
|------|------|
| `embed_search` | 시맨틱 폴더 탐색 — "나는 지금 어떤 상황인가?" |
| `grep_search` | 파일명+내용 정밀 탐색 — "무엇을 해야 하는가?" |
| `read_action` | 행동 파일의 전체 trajectory 읽기 |
| `write_action` | 새 경험 기록 (append-only, 네이밍 검증) |
| `suggest_location` | 새 행동을 기록할 폴더 위치 추천 |
| `create_folder` | 새 맥락 폴더 생성 + 임베딩 등록 |
| `move_action` | 파일/폴더 이동 + 인덱스 자동 갱신 |
| `rename_action` | 파일 이름 변경 + 네이밍 검증 |
| `create_habit` | 자주 쓰는 경로에 심볼릭 링크 단축키 |
| `get_warnings` | 네이밍 위반 경고 로그 조회 (성찰 자료) |

## 빠른 시작

### 빌드

```bash
cargo build --release
```

### 환경변수

```bash
export PHRONESIS_DATA_ROOT="$HOME/.phronesis"
export OPENAI_API_KEY="sk-..."
# 선택: export PHRONESIS_EMBEDDING_MODEL="text-embedding-3-small"
```

### Claude Code에서 사용

`.claude/settings.json`에 추가:

```json
{
  "mcpServers": {
    "phronesis": {
      "command": "/path/to/phronesis/target/release/phronesis",
      "env": {
        "PHRONESIS_DATA_ROOT": "/path/to/.phronesis",
        "OPENAI_API_KEY": "sk-..."
      }
    }
  }
}
```

### 에이전트 사용 플로우

```
1. embed_search("화난 사용자 대응")
   → /praxis/communication 폴더 앵커링

2. grep_search("praxis/communication", "화난|불만")
   → 기존 행동 파일 탐색

3a. (있으면) read_action("praxis/communication/empathize_calmly.jsonl")
   → 과거 경험 참조

3b. (없으면) suggest_location("화난 사용자에게 공감하며 대응")
   → write_action으로 새 행동 기록

4. 다음에 같은 상황 → 2단계에서 바로 찾아 참조
```

## 네이밍 규칙

- **폴더**: 소문자_언더바, 명사 중심 (예: `communication/formal_email`)
- **파일**: `동사_목적어_방식.jsonl` (예: `send_apology_with_alternative.jsonl`)
- 규칙 위반 시 **경고를 반환**하되 저장은 허용 (소프트 검증)
- 경고는 `/reflection/warnings/`에 누적되어 성찰 자료로 활용

## 기술 스택

| 컴포넌트 | 기술 |
|----------|------|
| 언어 | Rust |
| MCP SDK | rmcp 1.4 |
| 벡터 인덱스 | instant-distance (HNSW, in-memory) |
| 임베딩 | OpenAI text-embedding-3-small (추상화 레이어) |
| 데이터 포맷 | JSONL (append-only) |
| 프로토콜 | MCP over stdio |

## 테스트

```bash
# Unit tests (46개)
cargo test

# Unit + Integration tests (60개)
cargo test --features test-utils
```

## 설계 원칙

1. **Append-only 불변성** — 삭제/수정 API 없음. 경험은 불가역적
2. **Trait 기반 추상화** — 임베딩/벡터 저장소 교체 가능
3. **파일시스템이 곧 메모리** — HNSW 인덱스는 재구축 가능한 파생 캐시
4. **소프트 검증** — 네이밍 위반은 경고 + 저장 허용 + 로그 누적
5. **방어적 파싱** — 크래시 후 JSONL 부분 기록은 건너뛰고 경고 로그

## 라이선스

MIT

## 크레딧

아리스토텔레스의 실천적 지혜(Phronesis, φρόνησις)와 지행합일(知行合一)의 철학에서 영감을 받았습니다.
