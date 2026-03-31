use crate::cli_error::CliError;
use serde_json::{json, Value};

const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

pub async fn list_orgs(api_url: &str) -> Result<(), CliError> {
    let client = reqwest::Client::new();
    let payload: Value = client
        .get(format!("{api_url}/api/orgs"))
        .send()
        .await
        .map_err(|e| CliError::ApiCallFailed(format!("list orgs failed: {e}")))?
        .json()
        .await
        .map_err(|e| CliError::ApiCallFailed(format!("decode org list failed: {e}")))?;
    println!("{CYAN}ORG ID | STATUS | CEO | MEMBERS | BUDGET{RESET}");
    for org in payload["orgs"].as_array().into_iter().flatten() {
        let id = org["id"].as_str().unwrap_or_default();
        let details = fetch_org_detail(&client, api_url, id).await?;
        println!("{}", format_org_row(&details));
    }
    Ok(())
}

pub async fn show_org(id: &str, api_url: &str) -> Result<(), CliError> {
    let client = reqwest::Client::new();
    let detail = fetch_org_detail(&client, api_url, id).await?;
    let decisions: Value = client
        .get(format!("{api_url}/api/orgs/{id}/decisions"))
        .send()
        .await
        .map_err(|e| CliError::ApiCallFailed(format!("get decisions failed: {e}")))?
        .json()
        .await
        .map_err(|e| CliError::ApiCallFailed(format!("decode decisions failed: {e}")))?;
    let telemetry: Value = client
        .get(format!("{api_url}/api/orgs/{id}/telemetry?period=day"))
        .send()
        .await
        .map_err(|e| CliError::ApiCallFailed(format!("get telemetry failed: {e}")))?
        .json()
        .await
        .map_err(|e| CliError::ApiCallFailed(format!("decode telemetry failed: {e}")))?;
    let digest: Value = match client.get(format!("{api_url}/api/orgs/{id}/digest")).send().await {
        Ok(resp) => resp.json::<Value>().await.unwrap_or_else(|_| json!({"digest": null})),
        Err(_) => json!({"digest": null}),
    };
    let out = json!({
        "org": detail["org"],
        "members": detail["members"],
        "services": detail["services"],
        "recent_decisions": decisions["decisions"],
        "telemetry": telemetry["aggregate"],
        "latest_digest": digest["digest"]
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&out).unwrap_or_else(|_| out.to_string())
    );
    Ok(())
}

pub fn format_org_row(org_detail: &Value) -> String {
    let id = org_detail["org"]["id"].as_str().or_else(|| org_detail["id"].as_str()).unwrap_or("-");
    let status = org_detail["org"]["status"].as_str().or_else(|| org_detail["status"].as_str()).unwrap_or("-");
    let ceo = org_detail["org"]["ceo_agent"].as_str().or_else(|| org_detail["ceo_agent"].as_str()).unwrap_or("-");
    let members = org_detail["member_count"].as_u64().unwrap_or(0);
    let budget = org_detail["budget_usage_pct"].as_f64().unwrap_or(0.0);
    let status_colored = if status == "active" {
        format!("{GREEN}{status}{RESET}")
    } else {
        format!("{YELLOW}{status}{RESET}")
    };
    format!("{id} | {status_colored} | {ceo} | {members} | {budget:.1}%")
}

async fn fetch_org_detail(client: &reqwest::Client, api_url: &str, id: &str) -> Result<Value, CliError> {
    let detail: Value = client
        .get(format!("{api_url}/api/orgs/{id}"))
        .send()
        .await
        .map_err(|e| CliError::ApiCallFailed(format!("get org detail failed: {e}")))?
        .json()
        .await
        .map_err(|e| CliError::ApiCallFailed(format!("decode org detail failed: {e}")))?;
    let member_count = detail["members"].as_array().map(|m| m.len()).unwrap_or(0);
    let budget = detail["org"]["budget"].as_f64().unwrap_or(0.0).max(1.0);
    let agg: Value = match client
        .get(format!("{api_url}/api/orgs/{id}/telemetry?period=day"))
        .send()
        .await
    {
        Ok(resp) => resp
            .json::<Value>()
            .await
            .unwrap_or_else(|_| json!({"aggregate": {"cost": 0.0}})),
        Err(_) => json!({"aggregate": {"cost": 0.0}}),
    };
    let cost = agg["aggregate"]["cost"].as_f64().unwrap_or(0.0);
    Ok(json!({
        "org": detail["org"],
        "members": detail["members"],
        "services": detail["services"],
        "member_count": member_count,
        "budget_usage_pct": (cost / budget) * 100.0
    }))
}
