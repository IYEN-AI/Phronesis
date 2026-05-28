use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::error::{PhronesisError, Result};
use crate::types::{
    ArtifactKind, ArtifactStatus, Claim, Confidence, EvidenceEdge, EvidenceGraph, EvidenceRef,
    EvidenceRelation, Freshness, ProjectionArtifact, ProjectionArtifactListResponse,
    ProjectionEnqueueRequestPayload, ProjectionEnqueueResult, ProjectionErrorBody, ProjectionJob,
    ProjectionJobListResponse, ProjectionJobStatus, ProjectionKindFilter,
    ProjectionRefreshStaleRequest, ProjectionRefreshStaleResponse,
    ProjectionSearchArtifactsRequest, ProjectionSearchResponse, ProjectionSearchResult,
    Sensitivity, SourceKind,
};

pub fn ensure_layout(data_root: &Path) -> Result<()> {
    std::fs::create_dir_all(projection_root(data_root).join("artifacts"))?;
    std::fs::create_dir_all(projection_root(data_root).join("jobs"))?;
    Ok(())
}

pub fn enqueue(
    data_root: &Path,
    req: ProjectionEnqueueRequestPayload,
) -> Result<ProjectionEnqueueResult> {
    ensure_layout(data_root)?;
    let requested_at = Utc::now().to_rfc3339();
    let job_id = stable_id(
        "job",
        &format!("{}:{}", req.scope.agent, req.idempotency_key),
    );
    let artifacts = build_artifacts(data_root, &req)?;
    let artifact_ids = artifacts
        .iter()
        .map(|artifact| artifact.artifact_id.clone())
        .collect::<Vec<_>>();

    for artifact in &artifacts {
        write_artifact(data_root, artifact)?;
    }

    let job = ProjectionJob {
        job_id: job_id.clone(),
        agent: req.scope.agent.clone(),
        kind: req.kind.clone(),
        status: ProjectionJobStatus::Succeeded,
        requested_at,
        started_at: None,
        finished_at: Some(Utc::now().to_rfc3339()),
        retry_count: 0,
        retry_history: vec![],
        last_error: None,
        watermark_before: req.source_watermark.clone(),
        watermark_after: Some(req.source_watermark.clone()),
        stale: false,
        stale_reason: None,
        idempotency_key: req.idempotency_key.clone(),
        produced_artifact_ids: artifact_ids,
    };
    write_job(data_root, &job)?;

    Ok(ProjectionEnqueueResult {
        job_id,
        status: ProjectionJobStatus::Succeeded,
        accepted: true,
        idempotency_key: req.idempotency_key,
        existing_job_id: None,
    })
}

pub fn get_job(data_root: &Path, job_id: &str) -> Result<ProjectionJob> {
    read_json(&job_path(data_root, job_id))
}

pub fn list_jobs(
    data_root: &Path,
    agent: &str,
    kind: Option<ProjectionKindFilter>,
    status: Option<ProjectionJobStatus>,
    limit: usize,
    offset: usize,
) -> Result<ProjectionJobListResponse> {
    let mut jobs = read_dir_json::<ProjectionJob>(&jobs_dir(data_root))?
        .into_iter()
        .filter(|job| job.agent == agent)
        .filter(|job| kind.as_ref().is_none_or(|kind| job.kind == *kind))
        .filter(|job| status.as_ref().is_none_or(|status| job.status == *status))
        .collect::<Vec<_>>();
    jobs.sort_by(|a, b| b.requested_at.cmp(&a.requested_at));
    let total_count = jobs.len();
    let jobs = jobs.into_iter().skip(offset).take(limit).collect();
    Ok(ProjectionJobListResponse {
        jobs,
        total_count,
        limit,
        offset,
    })
}

pub fn get_artifact(data_root: &Path, artifact_id: &str) -> Result<ProjectionArtifact> {
    let kind = artifact_kind_from_id(artifact_id)?;
    read_json(&artifact_path(data_root, kind, artifact_id))
}

pub fn list_artifacts(
    data_root: &Path,
    agent: &str,
    kind: Option<ArtifactKind>,
    status: Option<ArtifactStatus>,
    granularity: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<ProjectionArtifactListResponse> {
    let mut artifacts = Vec::new();
    let kinds = match kind {
        Some(kind) => vec![kind],
        None => vec![
            ArtifactKind::Wisdom,
            ArtifactKind::Decision,
            ArtifactKind::TemporalRollup,
            ArtifactKind::Report,
        ],
    };

    for kind in kinds {
        let dir = artifacts_dir(data_root, kind.clone());
        if !dir.exists() {
            continue;
        }
        artifacts.extend(
            read_dir_json::<ProjectionArtifact>(&dir)?
                .into_iter()
                .filter(|artifact| artifact.agent == agent)
                .filter(|artifact| {
                    status
                        .as_ref()
                        .is_none_or(|status| artifact.status == *status)
                })
                .filter(|artifact| {
                    granularity.is_none_or(|granularity| {
                        artifact
                            .body
                            .get("granularity")
                            .and_then(|value| value.as_str())
                            .is_none_or(|value| value == granularity)
                    })
                }),
        );
    }

    artifacts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let total_count = artifacts.len();
    let artifacts = artifacts.into_iter().skip(offset).take(limit).collect();
    Ok(ProjectionArtifactListResponse {
        artifacts,
        total_count,
        limit,
        offset,
    })
}

pub fn search_artifacts(
    data_root: &Path,
    req: ProjectionSearchArtifactsRequest,
) -> Result<ProjectionSearchResponse> {
    let limit = req.limit.unwrap_or(20).max(1);
    let artifacts = list_artifacts(
        data_root,
        &req.agent,
        req.kind.clone(),
        None,
        None,
        usize::MAX,
        0,
    )?;
    let terms = normalized_terms(&req.q);
    let mut results = Vec::new();
    for artifact in artifacts.artifacts {
        let haystack =
            format!("{} {}", artifact.body, artifact.evidence_graph_text()).to_lowercase();
        if terms.iter().all(|term| haystack.contains(term)) {
            results.push(ProjectionSearchResult {
                artifact,
                score: 1.0,
                why: "matched projection artifact text".to_string(),
            });
        }
    }
    let total_count = results.len();
    results.truncate(limit);
    Ok(ProjectionSearchResponse {
        results,
        total_count,
    })
}

pub fn refresh_stale(
    data_root: &Path,
    req: ProjectionRefreshStaleRequest,
) -> Result<ProjectionRefreshStaleResponse> {
    let limit = req.limit.unwrap_or(100).max(1);
    let stale_artifacts = list_artifacts(
        data_root,
        &req.agent,
        kind_filter_to_artifact(req.kind.clone()),
        Some(ArtifactStatus::Stale),
        None,
        limit,
        0,
    )?;
    let mut enqueued_job_ids = Vec::new();
    for artifact in stale_artifacts.artifacts {
        let enqueue_req = ProjectionEnqueueRequestPayload {
            kind: artifact.kind.clone().into(),
            scope: crate::types::ProjectionScope {
                agent: req.agent.clone(),
                source_paths: artifact
                    .evidence_graph
                    .evidence
                    .iter()
                    .map(|e| e.path.clone())
                    .collect(),
                granularity: artifact
                    .body
                    .get("granularity")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                period_start: artifact
                    .body
                    .get("period_start")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                period_end: artifact
                    .body
                    .get("period_end")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
            },
            idempotency_key: artifact.idempotency_key.clone(),
            source_watermark: artifact.source_watermark.clone(),
        };
        enqueued_job_ids.push(enqueue(data_root, enqueue_req)?.job_id);
    }
    Ok(ProjectionRefreshStaleResponse {
        enqueued_job_ids,
        skipped_count: 0,
    })
}

pub fn ok_envelope<T: serde::Serialize>(result: T) -> String {
    serde_json::to_string_pretty(&crate::types::ProjectionEnvelope {
        ok: true,
        result: Some(result),
        error: None::<ProjectionErrorBody>,
    })
    .unwrap_or_default()
}

pub fn err_envelope(
    code: &str,
    message: impl Into<String>,
    retryable: bool,
    details: serde_json::Value,
) -> String {
    let details = details
        .as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect();
    serde_json::to_string_pretty(&crate::types::ProjectionEnvelope::<serde_json::Value> {
        ok: false,
        result: None,
        error: Some(ProjectionErrorBody {
            code: code.to_string(),
            message: message.into(),
            retryable,
            details,
        }),
    })
    .unwrap_or_default()
}

fn build_artifacts(
    data_root: &Path,
    req: &ProjectionEnqueueRequestPayload,
) -> Result<Vec<ProjectionArtifact>> {
    let mut artifacts = Vec::new();
    let source_documents = load_sources(data_root, &req.scope.agent, &req.scope.source_paths)?;
    let kinds = match req.kind {
        ProjectionKindFilter::All => vec![
            ArtifactKind::Wisdom,
            ArtifactKind::Decision,
            ArtifactKind::Report,
            ArtifactKind::TemporalRollup,
        ],
        ProjectionKindFilter::Wisdom => vec![ArtifactKind::Wisdom],
        ProjectionKindFilter::Decision => vec![ArtifactKind::Decision],
        ProjectionKindFilter::Report => vec![ArtifactKind::Report],
        ProjectionKindFilter::TemporalRollup => vec![ArtifactKind::TemporalRollup],
    };

    for kind in kinds {
        if kind == ArtifactKind::TemporalRollup && !should_emit_rollup(req, &source_documents) {
            continue;
        }
        artifacts.push(build_artifact(
            &req.scope.agent,
            &kind,
            req,
            &source_documents,
        ));
    }

    Ok(artifacts)
}

fn build_artifact(
    agent: &str,
    kind: &ArtifactKind,
    req: &ProjectionEnqueueRequestPayload,
    sources: &[SourceDocument],
) -> ProjectionArtifact {
    let created_at = Utc::now().to_rfc3339();
    let artifact_id = artifact_id_for(kind, &req.scope, sources);
    let evidence_graph = evidence_graph(agent, sources);
    let body = match kind {
        ArtifactKind::Wisdom => serde_json::json!({
            "title": title_from_sources(sources, "Wisdom"),
            "summary": summary_from_sources(sources),
            "recommended_action": recommended_action_from_sources(sources),
            "applies_when": ["Projection runtime artifacts are requested"],
            "avoid_when": ["Source evidence is stale"],
            "warnings": ["Review operator overlays may supersede this output"],
            "confidence": "high",
            "support_count": sources.len(),
            "contradiction_count": 0
        }),
        ArtifactKind::Decision => serde_json::json!({
            "title": title_from_sources(sources, "Decision"),
            "summary": summary_from_sources(sources),
            "decision_type": if explicit_decision_source(sources) { "explicit" } else { "inferred" },
            "decision_status": if explicit_decision_source(sources) { "canonical" } else { "candidate" },
            "confidence": "high",
            "decided_at": serde_json::Value::Null,
            "rationale": source_excerpt(sources),
            "outcome": "projection_persisted"
        }),
        ArtifactKind::TemporalRollup => serde_json::json!({
            "granularity": rollup_granularity(req, sources),
            "period_start": req.scope.period_start.clone().unwrap_or_else(|| current_date()),
            "period_end": req.scope.period_end.clone().unwrap_or_else(|| current_date()),
            "summary": summary_from_sources(sources),
            "themes": themes_from_sources(sources),
            "open_loops": open_loops_from_sources(sources),
            "important_events": important_events_from_sources(sources)
        }),
        ArtifactKind::Report => serde_json::json!({
            "title": title_from_sources(sources, "Report"),
            "scope": req.scope.source_paths.join(", "),
            "summary": summary_from_sources(sources),
            "sections": [{
                "heading": "Findings",
                "claim_ids": evidence_graph.claims.iter().map(|claim| claim.claim_id.clone()).collect::<Vec<_>>()
            }]
        }),
    };

    ProjectionArtifact {
        schema_version: 1,
        artifact_id,
        kind: kind.clone(),
        agent: agent.to_string(),
        created_at,
        source_watermark: req.source_watermark.clone(),
        idempotency_key: req.idempotency_key.clone(),
        content_hash: content_hash(kind.as_str(), &body),
        status: ArtifactStatus::Active,
        freshness: Freshness::Fresh,
        evidence_graph,
        body,
    }
}

#[derive(Debug, Clone)]
struct SourceDocument {
    path: String,
    excerpt: String,
    content: String,
}

fn load_sources(
    data_root: &Path,
    agent: &str,
    source_paths: &[String],
) -> Result<Vec<SourceDocument>> {
    let agent_root = data_root.parent().ok_or_else(|| {
        PhronesisError::Config("PHRONESIS_DATA_ROOT must be an agent/_phronesis directory".into())
    })?;
    let mut documents = Vec::new();
    for rel in source_paths {
        validate_source_path(rel)?;
        let path = agent_root.join(rel);
        let content = std::fs::read_to_string(&path).map_err(|e| {
            PhronesisError::NotFound(format!(
                "source for agent {agent} not found: {} ({e})",
                path.display()
            ))
        })?;
        documents.push(SourceDocument {
            path: rel.clone(),
            excerpt: truncate_chars(strip_frontmatter(&content), 240),
            content,
        });
    }
    Ok(documents)
}

fn evidence_graph(agent: &str, sources: &[SourceDocument]) -> EvidenceGraph {
    let mut claims = Vec::new();
    let mut evidence = Vec::new();
    let mut edges = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let claim_id = format!("claim_{}", index + 1);
        let evidence_id = format!("ev_{}", index + 1);
        claims.push(Claim {
            claim_id: claim_id.clone(),
            text: source.excerpt.clone(),
            scope: source.path.clone(),
            confidence: Confidence::High,
        });
        evidence.push(EvidenceRef {
            evidence_id: evidence_id.clone(),
            source_kind: source_kind_for_path(&source.path),
            agent: agent.to_string(),
            path: source.path.clone(),
            record_id: Some(stable_id("rec", &source.path)),
            span: None,
            excerpt: source.excerpt.clone(),
            timestamp: Some(Utc::now().to_rfc3339()),
            sensitivity: Sensitivity::Operator,
        });
        edges.push(EvidenceEdge {
            claim_id,
            evidence_id,
            relation: EvidenceRelation::Supports,
            weight: Some(1.0),
        });
    }
    EvidenceGraph {
        claims,
        evidence,
        edges,
    }
}

fn source_kind_for_path(path: &str) -> SourceKind {
    if path.starts_with("raw/semantic") {
        SourceKind::Semantic
    } else if path.starts_with("raw/temporal") {
        SourceKind::Temporal
    } else if path.starts_with("docs") {
        SourceKind::Docs
    } else {
        SourceKind::External
    }
}

fn should_emit_rollup(req: &ProjectionEnqueueRequestPayload, sources: &[SourceDocument]) -> bool {
    req.scope.granularity.is_some()
        || sources
            .iter()
            .any(|source| source.path.starts_with("raw/temporal/"))
}

fn rollup_granularity(req: &ProjectionEnqueueRequestPayload, sources: &[SourceDocument]) -> String {
    if let Some(granularity) = &req.scope.granularity {
        return granularity.clone();
    }
    for source in sources {
        if let Some(granularity) = source.path.split('/').nth(2) {
            if matches!(granularity, "daily" | "weekly" | "monthly") {
                return granularity.to_string();
            }
        }
    }
    "daily".to_string()
}

fn explicit_decision_source(sources: &[SourceDocument]) -> bool {
    sources.iter().any(|source| {
        source.path.contains("decision")
            || source.content.to_lowercase().contains("decision")
            || source.content.to_lowercase().contains("accepted")
    })
}

fn title_from_sources(sources: &[SourceDocument], fallback: &str) -> String {
    sources
        .first()
        .map(|source| {
            source
                .path
                .rsplit('/')
                .next()
                .unwrap_or(fallback)
                .trim_end_matches(".md")
                .replace('_', " ")
        })
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn summary_from_sources(sources: &[SourceDocument]) -> String {
    if sources.is_empty() {
        "No source content available".to_string()
    } else {
        sources
            .iter()
            .map(|source| source.excerpt.clone())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn recommended_action_from_sources(sources: &[SourceDocument]) -> String {
    sources
        .first()
        .map(|source| format!("Review and apply the guidance from {}", source.path))
        .unwrap_or_else(|| "Review the available source evidence".to_string())
}

fn source_excerpt(sources: &[SourceDocument]) -> Option<String> {
    sources.first().map(|source| source.excerpt.clone())
}

fn themes_from_sources(sources: &[SourceDocument]) -> Vec<String> {
    sources
        .iter()
        .map(|source| title_from_sources(std::slice::from_ref(source), "theme"))
        .collect()
}

fn open_loops_from_sources(sources: &[SourceDocument]) -> Vec<String> {
    sources
        .iter()
        .filter_map(|source| {
            if source.content.to_lowercase().contains("follow-up")
                || source.content.to_lowercase().contains("todo")
            {
                Some(format!("Review follow-up in {}", source.path))
            } else {
                None
            }
        })
        .collect()
}

fn important_events_from_sources(sources: &[SourceDocument]) -> Vec<String> {
    sources
        .iter()
        .map(|source| source.excerpt.clone())
        .collect()
}

fn projection_root(data_root: &Path) -> PathBuf {
    data_root.join("projections")
}

fn artifacts_dir(data_root: &Path, kind: ArtifactKind) -> PathBuf {
    projection_root(data_root)
        .join("artifacts")
        .join(kind.as_str())
}

fn jobs_dir(data_root: &Path) -> PathBuf {
    projection_root(data_root).join("jobs")
}

fn artifact_path(data_root: &Path, kind: ArtifactKind, artifact_id: &str) -> PathBuf {
    artifacts_dir(data_root, kind).join(format!("{artifact_id}.json"))
}

fn job_path(data_root: &Path, job_id: &str) -> PathBuf {
    jobs_dir(data_root).join(format!("{job_id}.json"))
}

fn write_artifact(data_root: &Path, artifact: &ProjectionArtifact) -> Result<()> {
    write_json(
        &artifact_path(data_root, artifact.kind.clone(), &artifact.artifact_id),
        artifact,
    )
}

fn write_job(data_root: &Path, job: &ProjectionJob) -> Result<()> {
    write_json(&job_path(data_root, &job.job_id), job)
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

fn read_dir_json<T: serde::de::DeserializeOwned>(dir: &Path) -> Result<Vec<T>> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut values = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        values.push(read_json(&path)?);
    }
    Ok(values)
}

fn stable_id(prefix: &str, value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{prefix}_{:016x}", hasher.finish())
}

fn content_hash(kind: &str, body: &serde_json::Value) -> String {
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    body.to_string().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn artifact_id_for(
    kind: &ArtifactKind,
    scope: &crate::types::ProjectionScope,
    sources: &[SourceDocument],
) -> String {
    let base = if let Some(path) = scope.source_paths.first() {
        sanitize_token(path)
    } else if let Some(source) = sources.first() {
        sanitize_token(&source.path)
    } else {
        stable_id("scope", &scope.agent)
    };
    match kind {
        ArtifactKind::Wisdom => format!("{}:cand_{base}", scope.agent),
        ArtifactKind::Decision => format!("{}:dec_{base}", scope.agent),
        ArtifactKind::TemporalRollup => format!(
            "{}:roll_{}_{base}",
            scope.agent,
            scope
                .granularity
                .clone()
                .unwrap_or_else(|| "daily".to_string())
        ),
        ArtifactKind::Report => format!("{}:rep_{base}", scope.agent),
    }
}

fn sanitize_token(value: &str) -> String {
    value
        .trim_end_matches(".md")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn strip_frontmatter(content: &str) -> &str {
    if let Some(rest) = content.strip_prefix("---\n") {
        if let Some(idx) = rest.find("\n---\n") {
            return &rest[idx + 5..];
        }
    }
    content
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn normalized_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|term| {
            term.trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
                .to_lowercase()
        })
        .filter(|term| !term.is_empty())
        .collect()
}

fn kind_filter_to_artifact(kind: ProjectionKindFilter) -> Option<ArtifactKind> {
    match kind {
        ProjectionKindFilter::Wisdom => Some(ArtifactKind::Wisdom),
        ProjectionKindFilter::Decision => Some(ArtifactKind::Decision),
        ProjectionKindFilter::TemporalRollup => Some(ArtifactKind::TemporalRollup),
        ProjectionKindFilter::Report => Some(ArtifactKind::Report),
        ProjectionKindFilter::All => None,
    }
}

fn validate_source_path(rel: &str) -> Result<()> {
    if rel.is_empty() || rel.starts_with('/') || rel.contains("..") || rel.contains('\\') {
        return Err(PhronesisError::Validation(format!(
            "unsafe source path: {rel}"
        )));
    }
    if !(rel.starts_with("raw/semantic/")
        || rel.starts_with("raw/temporal/")
        || rel.starts_with("docs/"))
    {
        return Err(PhronesisError::Validation(format!(
            "unsupported source path root: {rel}"
        )));
    }
    Ok(())
}

fn artifact_kind_from_id(artifact_id: &str) -> Result<ArtifactKind> {
    let local_id = artifact_id.rsplit(':').next().unwrap_or(artifact_id);
    if local_id.starts_with("cand_") {
        Ok(ArtifactKind::Wisdom)
    } else if local_id.starts_with("dec_") {
        Ok(ArtifactKind::Decision)
    } else if local_id.starts_with("roll_") {
        Ok(ArtifactKind::TemporalRollup)
    } else if local_id.starts_with("rep_") {
        Ok(ArtifactKind::Report)
    } else {
        Err(PhronesisError::NotFound(format!(
            "unknown artifact id: {artifact_id}"
        )))
    }
}

fn current_date() -> String {
    Utc::now().date_naive().to_string()
}

trait EvidenceGraphText {
    fn evidence_graph_text(&self) -> String;
}

impl EvidenceGraphText for ProjectionArtifact {
    fn evidence_graph_text(&self) -> String {
        let claims = self
            .evidence_graph
            .claims
            .iter()
            .map(|claim| claim.text.clone())
            .collect::<Vec<_>>()
            .join(" ");
        let evidence = self
            .evidence_graph
            .evidence
            .iter()
            .map(|evidence| evidence.excerpt.clone())
            .collect::<Vec<_>>()
            .join(" ");
        format!("{claims} {evidence}")
    }
}

impl From<ArtifactKind> for ProjectionKindFilter {
    fn from(value: ArtifactKind) -> Self {
        match value {
            ArtifactKind::Wisdom => ProjectionKindFilter::Wisdom,
            ArtifactKind::Decision => ProjectionKindFilter::Decision,
            ArtifactKind::TemporalRollup => ProjectionKindFilter::TemporalRollup,
            ArtifactKind::Report => ProjectionKindFilter::Report,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn projection_enqueue_persists_job_and_artifacts() {
        let tmp = TempDir::new().unwrap();
        let data_root = tmp.path().join("_phronesis");
        let source = tmp.path().join("raw/semantic/trace.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "# Trace\n\nProjection runtime test").unwrap();

        let req = ProjectionEnqueueRequestPayload {
            kind: ProjectionKindFilter::Wisdom,
            scope: crate::types::ProjectionScope {
                agent: "alice".to_string(),
                source_paths: vec!["raw/semantic/trace.md".to_string()],
                granularity: None,
                period_start: None,
                period_end: None,
            },
            idempotency_key: "alice:wisdom:test".to_string(),
            source_watermark: crate::types::SourceWatermark {
                content_hash: "hash".to_string(),
                latest_mtime: Utc::now().to_rfc3339(),
                source_count: 1,
            },
        };

        let result = enqueue(&data_root, req).unwrap();
        assert!(result.accepted);
        let job = get_job(&data_root, &result.job_id).unwrap();
        assert_eq!(job.status, ProjectionJobStatus::Succeeded);
        assert_eq!(job.produced_artifact_ids.len(), 1);
        let artifact = get_artifact(&data_root, &job.produced_artifact_ids[0]).unwrap();
        assert_eq!(artifact.kind, ArtifactKind::Wisdom);
    }

    #[test]
    fn projection_search_and_refresh_work() {
        let tmp = TempDir::new().unwrap();
        let data_root = tmp.path().join("_phronesis");
        let source = tmp.path().join("raw/temporal/daily/2026-05-28.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "deployment follow-up todo").unwrap();

        let req = ProjectionEnqueueRequestPayload {
            kind: ProjectionKindFilter::All,
            scope: crate::types::ProjectionScope {
                agent: "alice".to_string(),
                source_paths: vec!["raw/temporal/daily/2026-05-28.md".to_string()],
                granularity: Some("daily".to_string()),
                period_start: Some("2026-05-28".to_string()),
                period_end: Some("2026-05-28".to_string()),
            },
            idempotency_key: "alice:all:test".to_string(),
            source_watermark: crate::types::SourceWatermark {
                content_hash: "hash".to_string(),
                latest_mtime: Utc::now().to_rfc3339(),
                source_count: 1,
            },
        };
        enqueue(&data_root, req).unwrap();

        let search = search_artifacts(
            &data_root,
            crate::types::ProjectionSearchArtifactsRequest {
                agent: "alice".to_string(),
                q: "deployment follow-up".to_string(),
                kind: Some(ArtifactKind::TemporalRollup),
                limit: Some(10),
            },
        )
        .unwrap();
        assert_eq!(search.total_count, 1);

        let artifacts = list_artifacts(
            &data_root,
            "alice",
            Some(ArtifactKind::TemporalRollup),
            None,
            Some("daily"),
            10,
            0,
        )
        .unwrap();
        assert_eq!(artifacts.total_count, 1);

        let refresh = refresh_stale(
            &data_root,
            crate::types::ProjectionRefreshStaleRequest {
                agent: "alice".to_string(),
                kind: ProjectionKindFilter::All,
                limit: Some(10),
            },
        )
        .unwrap();
        assert!(refresh.enqueued_job_ids.is_empty());
    }

    #[test]
    fn projection_enqueue_rejects_unsafe_source_paths() {
        let tmp = TempDir::new().unwrap();
        let data_root = tmp.path().join("_phronesis");
        let req = ProjectionEnqueueRequestPayload {
            kind: ProjectionKindFilter::Wisdom,
            scope: crate::types::ProjectionScope {
                agent: "alice".to_string(),
                source_paths: vec!["../bob/raw/semantic/secret.md".to_string()],
                granularity: None,
                period_start: None,
                period_end: None,
            },
            idempotency_key: "alice:unsafe".to_string(),
            source_watermark: crate::types::SourceWatermark {
                content_hash: "hash".to_string(),
                latest_mtime: Utc::now().to_rfc3339(),
                source_count: 1,
            },
        };
        let error = enqueue(&data_root, req).unwrap_err();
        assert!(error.to_string().contains("unsafe source path"));
    }
}
