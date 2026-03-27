#!/bin/bash
# Shared JSON protocol for worker communication.
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

AGENT_PROTOCOL_CONTEXT_BUDGET="${AGENT_PROTOCOL_CONTEXT_BUDGET:-2000}"
DAEMON_URL="${DAEMON_URL:-http://localhost:8420}"

_ap_tokens() { local p="${1-}"; [[ -n "$p" ]] || p='{}'; printf '%s' "$p" | python3 -c 'import sys,math; print(max(0,math.ceil(len(sys.stdin.read())/4)))'; }
_ap_meta() {
	local t="${1-}" d="${2-}" tc="${3-}"; [[ -n "$tc" ]] || tc='{}'
	python3 - "$t" "$d" "$tc" <<'PY'
import json,re,sys
t,d,raw=sys.argv[1],sys.argv[2],sys.argv[3]
try: tc=json.loads(raw) if raw else {}
except Exception: tc={}
fm=re.search(r"\|\s*Files:\s*([^|]+)",d,re.I); rm=re.search(r"\|\s*Ref:\s*([^|]+)",d,re.I)
files=[x.strip() for x in (fm.group(1).split(",") if fm else []) if x.strip()]
ref=rm.group(1).strip() if rm else ""
blocked=[]
for k in ("blockedBy","blocked_by","dependsOn","depends_on"):
    v=tc.get(k)
    if isinstance(v,list): blocked += [str(x).strip() for x in v if str(x).strip()]
    elif isinstance(v,str) and v.strip(): blocked.append(v.strip())
if not blocked and d:
    bm=re.search(r"blockedBy\s*[:=]\s*([^|]+)",d,re.I)
    if bm: blocked += [x.strip() for x in bm.group(1).split(",") if x.strip()]
print(json.dumps({"do":(t or re.sub(r"\s+\|\s*Files:.*$","",d).strip()),"files":files,"ref":ref,"blocked_by":sorted(set(blocked))},separators=(",",":")))
PY
}
_ap_rules() {
	local cfg="${2:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/config/orchestrator.yaml}"
	[[ -f "$cfg" ]] || {
		echo '{}'
		return
	}
	python3 - "$cfg" "${1:-}" <<'PY'
import json,sys
try: import yaml
except Exception: print("{}"); raise SystemExit(0)
d=yaml.safe_load(open(sys.argv[1],encoding="utf-8")) or {}; p=sys.argv[2]
pr=(d.get("projects",{}) or {}).get(p,{}) if isinstance(d,dict) else {}
r=d.get("routing",{}) if isinstance(d,dict) else {}
out={"project":pr if isinstance(pr,dict) else {}}
if isinstance(r,dict):
    out["routing"]={}
    bt=r.get("by_type",{})
    if isinstance(bt,dict): out["routing"]["by_type"]=bt
    bp=r.get("by_privacy",{}); priv=out["project"].get("privacy") if isinstance(out["project"],dict) else None
    if priv and isinstance(bp,dict): out["routing"]["privacy_rule"]=bp.get(priv,{})
print(json.dumps(out,separators=(",",":")))
PY
}
build_task_envelope() {
	local db_task_id="${1:?task db_id required}"
	# Fetch plan JSON via daemon API, extract task by db id
	local plan_json task_json
	# First get the plan_id for this task via the daemon
	task_json=$(curl -sf "${DAEMON_URL}/api/plan-db/task/${db_task_id}" 2>/dev/null) || {
		# Fallback: search across plans for this task db id
		# TODO: needs daemon endpoint for direct task-by-db-id lookup
		local plan_list plan_id_found=""
		plan_list=$(curl -sf "${DAEMON_URL}/api/plan-db/list" | jq -r '.[].id' 2>/dev/null) || return 1
		for pid in $plan_list; do
			task_json=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}" | jq -c ".tasks[] | select(.db_id==${db_task_id} or .id==${db_task_id})" 2>/dev/null)
			if [[ -n "$task_json" ]]; then
				plan_id_found="$pid"
				plan_json=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${pid}")
				break
			fi
		done
		[[ -n "$task_json" ]] || return 1
	}
	# Extract fields from task JSON
	local task_id plan_id project_id wave_id_fk title description test_criteria worktree
	task_id=$(echo "$task_json" | jq -r '.task_id // ""')
	plan_id=$(echo "$task_json" | jq -r '.plan_id // 0')
	project_id=$(echo "$task_json" | jq -r '.project_id // ""')
	wave_id_fk=$(echo "$task_json" | jq -r '.wave_id_fk // 0')
	title=$(echo "$task_json" | jq -r '.title // ""')
	description=$(echo "$task_json" | jq -r '.description // ""')
	test_criteria=$(echo "$task_json" | jq -cr '.test_criteria // "{}"')
	# Get worktree from plan data
	if [[ -n "${plan_json:-}" ]]; then
		worktree=$(echo "$plan_json" | jq -r '.worktree_path // ""')
	else
		plan_json=$(curl -sf "${DAEMON_URL}/api/plan-db/json/${plan_id}" 2>/dev/null) || plan_json='{}'
		worktree=$(echo "$plan_json" | jq -r '.worktree_path // ""')
	fi
	local meta do_field files_json ref_field blocked_by_json verify_json
	meta=$(_ap_meta "$title" "$description" "$test_criteria")
	do_field=$(echo "$meta" | jq -r '.do // ""'); files_json=$(echo "$meta" | jq -c '.files // []')
	ref_field=$(echo "$meta" | jq -r '.ref // ""'); blocked_by_json=$(echo "$meta" | jq -c '.blocked_by // []')
	verify_json=$(echo "$test_criteria" | jq -c '.verify // []' 2>/dev/null || echo '[]')
	local wave_siblings_json='[]' prior_outputs_json='[]'
	if [[ "$wave_id_fk" != "0" && "$wave_id_fk" != "null" ]]; then
		# Get wave siblings from plan JSON
		wave_siblings_json=$(echo "$plan_json" | jq -c "[.tasks[] | select(.wave_id_fk==${wave_id_fk} and (.db_id!=${db_task_id} and .id!=${db_task_id})) | .task_id]" 2>/dev/null || echo '[]')
	fi
	if [[ "$(echo "$blocked_by_json" | jq 'length')" -gt 0 ]]; then
		# Get prior outputs from plan JSON for blocked-by tasks
		prior_outputs_json=$(echo "$plan_json" | jq -c --argjson blocked "$blocked_by_json" '[.tasks[] | select(.task_id as $tid | $blocked | index($tid)) | select(.output_data != null and .output_data != "") | {task_id, output_data}]' 2>/dev/null || echo '[]')
	fi
	local context_json project_rules_json; project_rules_json=$(_ap_rules "$project_id")
	context_json=$(jq -cn --argjson wave "$wave_siblings_json" --argjson prior "$prior_outputs_json" --argjson rules "$project_rules_json" '{wave_siblings:$wave,prior_outputs:$prior,project_rules:$rules}')
	while [[ $(_ap_tokens "$context_json") -gt "$AGENT_PROTOCOL_CONTEXT_BUDGET" ]]; do
		if [[ "$(echo "$context_json" | jq '.prior_outputs|length')" -gt 0 ]]; then
			context_json=$(echo "$context_json" | jq '.prior_outputs |= .[:-1]')
		elif [[ "$(echo "$context_json" | jq '.project_rules.routing? != null')" == "true" ]]; then
			context_json=$(echo "$context_json" | jq '.project_rules={project:(.project_rules.project // {})}')
		else
			context_json=$(echo "$context_json" | jq '.project_rules={}')
			break
		fi
	done
	jq -cn --arg task_id "$task_id" --argjson plan_id "$plan_id" --arg project_id "$project_id" --arg worktree "$worktree" --arg do "$do_field" --argjson files "$files_json" --argjson verify "$verify_json" --arg ref "$ref_field" --argjson context "$context_json" '{task_id:$task_id,plan_id:$plan_id,project_id:$project_id,worktree:$worktree,do:$do,files:$files,verify:$verify,ref:$ref,context:$context}'
}
parse_worker_result() {
	local src="${1-}" payload; [[ -n "$src" ]] || src='{}'; payload="$src"; [[ -f "$src" ]] && payload="$(<"$src")"
	printf '%s' "$payload" | jq -c '
		def arr(v): if v==null then [] elif (v|type)=="array" then v elif (v|type)=="string" then [v] else [] end;
		(if type=="object" then . else {} end) as $r |
		{exit_code:($r.exit_code // $r.code // $r.result.exit_code // (if (($r.error // $r.stderr // "") != "") then 1 else 0 end)),
		 files_modified:((arr($r.files_modified)+arr($r.files)+arr($r.artifacts)+arr($r.result.files_modified))|unique),
		 tokens_used:($r.tokens_used // $r.tokens // $r.result.tokens_used // $r.usage.total_tokens // 0),
		 error:($r.error // $r.stderr // $r.result.error // ""),
		 summary:($r.summary // $r.result.summary // $r.message // "")}'
}
format_thor_input() {
	local env_src="${1-}" worker_src="${2-}" envelope
	[[ -n "$env_src" ]] || env_src='{}'; [[ -n "$worker_src" ]] || worker_src='{}'
	envelope="$env_src"; [[ -f "$env_src" ]] && envelope="$(<"$env_src")"
	local result; result=$(parse_worker_result "$worker_src")
	jq -cn --argjson env "$envelope" --argjson res "$result" '{
		task_id:($env.task_id // ""), plan_id:($env.plan_id // ""), project_id:($env.project_id // ""),
		do:($env.do // ""), files:($env.files // []), verify:($env.verify // []), ref:($env.ref // ""),
		context:{wave_siblings:($env.context.wave_siblings // []),
			prior_outputs:(($env.context.prior_outputs // []) | map({task_id:(.task_id // ""),summary:(.output_data.summary // .summary // "")})),
			project_rules:($env.context.project_rules // {})}, result:$res}'
}
