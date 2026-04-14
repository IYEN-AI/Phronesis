use std::path::Path;

use crate::error::Result;
use crate::types::MetaEntry;

const PILLAR_SEEDS: &[(&str, &str)] = &[
    ("self", "에이전트 자기 유지와 정체성 관리"),
    ("perception", "외부 상황 인지와 데이터 해석"),
    ("cognition", "내부 추론과 지식 처리 프로세스"),
    ("praxis", "외부 세계에 영향을 미치는 실천 행동"),
    ("evolution", "새로운 능력 획득과 구조 확장"),
    ("reflection", "기존 지식 재구성과 자기 성찰"),
];

const SKILLS_SEED: &str = r#"# Phronesis Skills Guide

이 문서는 Phronesis 에이전틱 메모리 시스템의 사용 가이드입니다.

## 6-Pillar 분류 체계

| Pillar | 용도 | 기록 대상 |
|--------|------|----------|
| `/self` | 자기 유지, 정체성 | 시스템 상태, 페르소나, 보안 관련 행동 |
| `/perception` | 상황 인지 | 사용자 의도 분석, 환경 모니터링 행동 |
| `/cognition` | 내부 추론 | 논리 연산, 시뮬레이션, 지식 인출 행동 |
| `/praxis` | 외부 실천 | 도구 실행, 대화, 콘텐츠 생성 행동 |
| `/evolution` | 능력 확장 | 새로운 행동 생성, 폴더 구조 확장 |
| `/reflection` | 성찰 | 파일 재배치/재명명, 피드백 기록, 경고 리뷰 |

## 네이밍 규칙

- **폴더**: 소문자_언더바, 명사 중심 맥락 (예: `communication/formal_email`)
- **파일**: `동사_목적어_방식.jsonl` (예: `send_apology_with_alternative.jsonl`)

## 탐색 패턴

1. `embed_search`로 상황 인식 — "나는 지금 어떤 맥락에 있는가"
2. `grep_search`로 행동 선택 — "무엇을 해야 하는가"
3. 기존 행동이 없으면 `suggest_location` → `write_action`으로 새 행동 기록

## 서브에이전트 패턴 (추천)

메모리 탐색은 메인 에이전트의 컨텍스트를 오염시킬 수 있습니다.
서브에이전트를 포크하여 탐색을 수행하고, 클린 결과만 메인 에이전트에 반환하는 것을 권장합니다.

## 성찰 트리거

같은 행동을 세션 중 3번 이상 반복하고 있다면, 성찰을 수행하세요:
1. `get_warnings`로 경고 로그를 확인
2. 반복 패턴을 분석
3. 필요시 `move_action`/`rename_action`으로 구조 개선

<!-- AGENT APPENDABLE SECTION -->
<!-- 아래에 에이전트가 경험을 통해 학습한 내용을 추가합니다 -->
"#;

pub fn bootstrap(data_root: &Path) -> Result<()> {
    // Create 6-Pillar directories with .meta.jsonl
    for (name, description) in PILLAR_SEEDS {
        let pillar_dir = data_root.join(name);
        std::fs::create_dir_all(&pillar_dir)?;

        let meta_path = pillar_dir.join(".meta.jsonl");
        if !meta_path.exists() {
            let entry = MetaEntry {
                description: description.to_string(),
                created: Some(chrono::Utc::now().to_rfc3339()),
                updated: None,
            };
            let line = serde_json::to_string(&entry)?;
            std::fs::write(&meta_path, format!("{}\n", line))?;
        }
    }

    // Create warnings directory
    let warnings_dir = data_root.join("reflection/warnings");
    std::fs::create_dir_all(&warnings_dir)?;
    let warnings_file = warnings_dir.join("naming_violations.jsonl");
    if !warnings_file.exists() {
        std::fs::write(&warnings_file, "")?;
    }

    // Create skills.md
    let skills_path = data_root.join("skills.md");
    if !skills_path.exists() {
        std::fs::write(&skills_path, SKILLS_SEED)?;
    }

    // Create .index directory
    let index_dir = data_root.join(".index");
    std::fs::create_dir_all(&index_dir)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_bootstrap_creates_pillars() {
        let tmp = TempDir::new().unwrap();
        bootstrap(tmp.path()).unwrap();

        for (name, _) in PILLAR_SEEDS {
            assert!(tmp.path().join(name).is_dir());
            assert!(tmp.path().join(name).join(".meta.jsonl").exists());
        }
        assert!(tmp.path().join("skills.md").exists());
        assert!(tmp
            .path()
            .join("reflection/warnings/naming_violations.jsonl")
            .exists());
        assert!(tmp.path().join(".index").is_dir());
    }

    #[test]
    fn test_bootstrap_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        bootstrap(tmp.path()).unwrap();
        bootstrap(tmp.path()).unwrap();
    }

    #[test]
    fn test_bootstrap_partial_recovery() {
        let tmp = TempDir::new().unwrap();

        std::fs::create_dir_all(tmp.path().join("self")).unwrap();
        std::fs::create_dir_all(tmp.path().join("praxis")).unwrap();

        bootstrap(tmp.path()).unwrap();

        for (name, _) in PILLAR_SEEDS {
            assert!(tmp.path().join(name).is_dir());
            assert!(tmp.path().join(name).join(".meta.jsonl").exists());
        }
        assert!(tmp.path().join("skills.md").exists());
    }

    #[test]
    fn test_bootstrap_meta_content_valid_json() {
        let tmp = TempDir::new().unwrap();
        bootstrap(tmp.path()).unwrap();

        for (name, _) in PILLAR_SEEDS {
            let meta_path = tmp.path().join(name).join(".meta.jsonl");
            let content = std::fs::read_to_string(&meta_path).unwrap();
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let parsed: serde_json::Value = serde_json::from_str(line)
                    .unwrap_or_else(|e| panic!("Invalid JSON in {}: {} — line: {}", name, e, line));
                assert!(
                    parsed.get("description").is_some(),
                    "Meta entry in {} should have 'description' field",
                    name
                );
            }
        }
    }

    #[test]
    fn test_bootstrap_skills_contains_all_pillars() {
        let tmp = TempDir::new().unwrap();
        bootstrap(tmp.path()).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("skills.md")).unwrap();
        for (name, _) in PILLAR_SEEDS {
            assert!(
                content.contains(&format!("/{}", name)),
                "skills.md should mention /{}",
                name
            );
        }
    }
}
