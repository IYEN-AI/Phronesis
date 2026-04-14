# ARD — Phronesis × MemoryOwn 통합

## 1. 결정

Phronesis를 `phronesis-core` 라이브러리 크레이트로 분리하고, MemoryOwn 백엔드가 이를 의존성으로 사용한다. MemoryOwn의 semantic 영역 백엔드를 Phronesis가 담당한다.

## 2. 아키텍처

```
MemoryOwn (axum HTTP API + 웹 프론트엔드)
│
├── temporal/*  → MemoryOwn 자체 (마크다운, grep 검색)
├── docs/*      → MemoryOwn 자체 (마크다운, grep 검색)
└── semantic/*  → phronesis-core (JSONL, 임베딩+grep 하이브리드 검색)
                   ├── EmbeddingProvider (로컬 MultilingualE5Small)
                   ├── HnswStore (벡터 인덱스)
                   ├── Action CRUD (append-only JSONL)
                   └── Naming validation + warnings
```

## 3. Phronesis 레포 구조 변경

### Before (v0.1)
```
Phronesis/
├── Cargo.toml          # 단일 바이너리
└── src/
    ├── main.rs         # MCP 서버 진입점
    ├── server.rs       # MCP 도구 정의
    └── ...             # 핵심 로직
```

### After (v0.2)
```
Phronesis/
├── Cargo.toml          # workspace
├── phronesis-core/     # 라이브러리 크레이트 (MemoryOwn이 사용)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── config.rs
│       ├── types.rs
│       ├── error.rs
│       ├── fs/         # action, meta, naming, warnings, skills
│       ├── search/     # embedding, vector_store, grep, suggest
│       └── evolution/  # move, rename, habit
├── phronesis-mcp/      # MCP 서버 (기존 독립 실행용)
│   ├── Cargo.toml      # depends on phronesis-core
│   └── src/
│       ├── main.rs
│       └── server.rs
└── docs/
```

## 4. phronesis-core 공개 API

```rust
// -- 초기화 --
pub struct Phronesis {
    config: Config,
    store: HnswStore,
    provider: Box<dyn EmbeddingProvider>,
}

impl Phronesis {
    pub async fn new(config: Config) -> Result<Self>;
    pub async fn new_with_provider(config: Config, provider: impl EmbeddingProvider) -> Result<Self>;
}

// -- 검색 --
impl Phronesis {
    /// 시맨틱 임베딩 검색 — "어떤 상황인가?"
    pub async fn embed_search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>>;
    
    /// 파일명+내용 grep — "뭘 해야 하는가?"
    pub fn grep_search(&self, folder: &str, pattern: &str, max_results: usize) -> Result<Vec<GrepResult>>;
    
    /// 하이브리드: embed → grep 2단계
    pub async fn hybrid_search(&self, query: &str, top_k: usize) -> Result<Vec<HybridResult>>;
}

// -- CRUD --
impl Phronesis {
    pub fn read_action(&self, path: &str) -> Result<Vec<ActionEntry>>;
    pub fn write_action(&self, path: &str, entry: &ActionEntry) -> Result<WriteResult>;
    pub fn suggest_location(&self, description: &str) -> Result<Vec<SuggestedLocation>>;
    pub fn create_folder(&mut self, path: &str, description: &str) -> Result<()>;
}

// -- 진화 --
impl Phronesis {
    pub fn move_action(&mut self, from: &str, to: &str) -> Result<()>;
    pub fn rename_action(&mut self, path: &str, new_name: &str) -> Result<RenameResult>;
    pub fn create_habit(&self, name: &str, target: &str) -> Result<()>;
}

// -- 성찰 --
impl Phronesis {
    pub fn get_warnings(&self, since: Option<DateTime>) -> Result<Vec<Warning>>;
}
```

## 5. MemoryOwn 통합 방식

### Cargo.toml
```toml
[dependencies]
phronesis-core = { git = "https://github.com/IYEN-AI/Phronesis", branch = "main" }
```

### MemoryOwn 백엔드 변경

```rust
// state.rs
pub struct AppState {
    pub config: AppConfig,
    pub phronesis: Arc<RwLock<Phronesis>>,  // 추가
}

// routes/search.rs — semantic scope일 때 Phronesis 사용
pub async fn search_agent(...) {
    match scope {
        "temporal" | "docs" => existing_grep_search(...),
        "semantic" => {
            let phronesis = state.phronesis.read().await;
            phronesis.hybrid_search(&query, limit).await
        }
        "all" => {
            // grep(temporal+docs) + phronesis(semantic) 합산
        }
    }
}

// routes/memory.rs — semantic PUT일 때 Phronesis에 기록
pub async fn write_semantic(...) {
    let mut phronesis = state.phronesis.write().await;
    phronesis.write_action(&path, &entry)?;
    // 기존 마크다운 저장도 병행 (호환성)
}
```

## 6. 마이그레이션

### 기존 semantic .md 파일 → Phronesis .jsonl

```
raw/semantic/ccapi-proxy.md
  → phronesis/praxis/infrastructure/manage_ccapi_proxy.jsonl

raw/semantic/lessons-learned.md  
  → phronesis/reflection/lessons/record_operational_lessons.jsonl
```

변환기를 만들어서 기존 .md 내용을 JSONL entry로 변환:
```jsonl
{"ts":"2026-04-10T...","situation":"원본 md에서 추출","action":"...","outcome":"..."}
```

## 7. Phase 계획

| Phase | 작업 | 규모 |
|-------|------|------|
| 1 | Phronesis workspace 분리 (core + mcp) | ~리팩토링 |
| 2 | phronesis-core 공개 API 정리 | ~100줄 |
| 3 | MemoryOwn에 phronesis-core 의존성 추가 + semantic 라우팅 | ~200줄 |
| 4 | 기존 semantic .md → .jsonl 마이그레이션 도구 | ~100줄 |
| 5 | 프론트엔드에서 semantic 검색 결과 표시 | ~UI 작업 |

## 8. 리스크

| 리스크 | 대응 |
|--------|------|
| 임베딩 모델 100MB 다운로드 | 첫 실행 시 한번만. 이후 캐시 |
| HNSW 메모리 사용량 | 수천 파일까지는 수 MB 수준 |
| .md ↔ .jsonl 이중 포맷 | Phase 4에서 마이그레이션. 과도기에는 병행 |
| phronesis-core API 변경 | 시맨틱 버저닝 + git tag |
