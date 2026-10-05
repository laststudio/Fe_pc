use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::Read;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use flate2::read::DeflateDecoder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapInstrumentationResult {
    pub nonce: String,
    pub state: HashMap<String, i32>,
    pub timestamp: u64,
}

/// FNV-1a 32-bit hash function matching FeCapSolver.kt
pub fn fnv1a(str: &str) -> i32 {
    let mut h: i32 = 0x811c9dc5u32 as i32;
    for ch in str.chars() {
        let code = ch as u32 as i32;
        h ^= code;
        let term = h
            .wrapping_shl(1)
            .wrapping_add(h.wrapping_shl(4))
            .wrapping_add(h.wrapping_shl(7))
            .wrapping_add(h.wrapping_shl(8))
            .wrapping_add(h.wrapping_shl(24));
        h = h.wrapping_add(term);
    }
    h
}

/// 32-bit xorshift PRNG generating lowercase hex string
pub fn generate_prng_hex(seed: &str, length: usize) -> String {
    let mut state = fnv1a(seed) as u32;
    let mut sb = String::with_capacity(length + 8);
    while sb.len() < length {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let _ = write!(sb, "{:08x}", state);
    }
    sb.truncate(length);
    sb
}

/// Solve a single proof-of-work challenge matching target prefix
pub fn solve_one(salt: &str, target: &str) -> u64 {
    let o = 4 * target.len();
    let a = o / 8;
    let l = o % 8;
    let padded_target = if target.len().is_multiple_of(2) {
        target.to_string()
    } else {
        format!("{target}0")
    };
    let c = padded_target.len() / 2;
    let mut f = Vec::with_capacity(c);
    for e in 0..c {
        let byte_val = u8::from_str_radix(&padded_target[2 * e..2 * e + 2], 16).unwrap_or(0);
        f.push(byte_val);
    }
    let g: u8 = if l > 0 {
        ((0xFFu16 << (8 - l)) & 0xFF) as u8
    } else {
        0
    };

    let mut base_hasher = Sha256::new();
    base_hasher.update(salt.as_bytes());

    let mut nonce_buf = [0u8; 32];
    let mut nonce: u64 = 0;

    loop {
        let mut hasher = base_hasher.clone();
        let len = format_u64_ascii(nonce, &mut nonce_buf);
        hasher.update(&nonce_buf[..len]);
        let hash = hasher.finalize();

        let mut match_found = true;
        for e in 0..a {
            if hash[e] != f[e] {
                match_found = false;
                break;
            }
        }
        if match_found && l > 0 && (hash[a] & g) != (f[a] & g) {
            match_found = false;
        }
        if match_found {
            return nonce;
        }
        nonce += 1;
    }
}

fn format_u64_ascii(mut value: u64, buf: &mut [u8; 32]) -> usize {
    if value == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut temp = [0u8; 20];
    let mut len = 0;
    while value > 0 {
        temp[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    for i in 0..len {
        buf[i] = temp[len - 1 - i];
    }
    len
}

/// Multi-threaded Proof-of-Work solver
pub fn solve_pow(token: &str, count: usize, salt_len: usize, target_len: usize) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }
    let num_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(count);

    let token = token.to_string();
    let chunk_size = count.div_ceil(num_threads);

    let mut handles = Vec::new();
    for thread_idx in 0..num_threads {
        let start = thread_idx * chunk_size + 1; // 1-based index (1..=count)
        let end = ((thread_idx + 1) * chunk_size).min(count);
        if start > count {
            break;
        }
        let token_clone = token.clone();
        handles.push(thread::spawn(move || {
            let mut results = Vec::with_capacity(end - start + 1);
            for t in start..=end {
                let salt = generate_prng_hex(&format!("{token_clone}{t}"), salt_len);
                let target = generate_prng_hex(&format!("{token_clone}{t}d"), target_len);
                let nonce = solve_one(&salt, &target);
                results.push((t, nonce.to_string()));
            }
            results
        }));
    }

    let mut all_results = Vec::with_capacity(count);
    for h in handles {
        if let Ok(res) = h.join() {
            all_results.extend(res);
        }
    }
    all_results.sort_by_key(|&(t, _)| t);
    all_results.into_iter().map(|(_, nonce_str)| nonce_str).collect()
}

/// Node chain DOM mock simulation and sum % 256
pub fn dom_sum_mock(x: i32, y: i32, z: i32) -> i32 {
    struct Node {
        parent: Option<usize>,
        inner_text: i32,
    }

    let mut nodes: Vec<Node> = Vec::with_capacity(30);
    nodes.push(Node {
        parent: None,
        inner_text: 0,
    });

    let build_chain = |parent_idx: usize, val_arg: i32, nodes: &mut Vec<Node>| -> usize {
        let mut cur = parent_idx;
        let mut v = val_arg;
        for _ in 0..8 {
            let child_idx = nodes.len();
            nodes.push(Node {
                parent: Some(cur),
                inner_text: v,
            });
            if (v & 1) == 0 {
                cur = child_idx;
            }
            v >>= 1;
        }
        cur
    };

    let leaf = build_chain(0, x, &mut nodes);
    let leaf = build_chain(leaf, y, &mut nodes);
    let leaf = build_chain(leaf, z, &mut nodes);

    let mut sum: i32 = 0;
    let mut cur = Some(leaf);
    while let Some(idx) = cur {
        if idx == 0 {
            break;
        }
        sum = sum.wrapping_add(nodes[idx].inner_text);
        cur = nodes[idx].parent;
    }

    sum % 256
}

fn fn_helper(a: i32, b: i32, c: i32) -> i32 {
    (b ^ a) | (c ^ b)
}

/// Decompress raw deflate data (RFC 1951 without zlib header)
pub fn decompress_raw_deflate(compressed: &[u8]) -> Result<String, String> {
    let mut decoder = DeflateDecoder::new(compressed);
    let mut s = String::new();
    decoder
        .read_to_string(&mut s)
        .map_err(|e| format!("Deflate解压失败: {e}"))?;
    Ok(s)
}

/// Helper to decode base64 strings with or without padding
pub fn decode_base64_instrumentation(b64: &str) -> Result<Vec<u8>, String> {
    let trimmed = b64.trim();
    let padded = match trimmed.len() % 4 {
        2 => format!("{trimmed}=="),
        3 => format!("{trimmed}="),
        _ => trimmed.to_string(),
    };
    base64::engine::general_purpose::STANDARD
        .decode(&padded)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(&padded))
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(trimmed))
        .map_err(|e| format!("Base64解码失败: {e}"))
}

/// Parse and execute the challenge instrumentation JS script
pub fn solve_instrumentation(script_text: &str) -> Result<CapInstrumentationResult, String> {
    let nonce_regex = Regex::new(r#"nonce:\s*["']([a-f0-9]+)["']"#).map_err(|e| e.to_string())?;
    let nonce = nonce_regex
        .captures(script_text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "Cannot find nonce in instrumentation".to_string())?;

    let var_decl_regex = Regex::new(
        r"var\s+([a-zA-Z0-9_$]+)=(\d+);var\s+([a-zA-Z0-9_$]+)=(\d+);var\s+([a-zA-Z0-9_$]+)=(\d+);var\s+([a-zA-Z0-9_$]+)=(\d+);",
    )
    .map_err(|e| e.to_string())?;
    let var_decl_match = var_decl_regex
        .captures(script_text)
        .ok_or_else(|| "Cannot find variable declarations in instrumentation".to_string())?;

    let vars = [
        var_decl_match[1].to_string(),
        var_decl_match[3].to_string(),
        var_decl_match[5].to_string(),
        var_decl_match[7].to_string(),
    ];

    let mut vals = HashMap::new();
    vals.insert(
        vars[0].clone(),
        var_decl_match[2]
            .parse::<i32>()
            .map_err(|e| e.to_string())?,
    );
    vals.insert(
        vars[1].clone(),
        var_decl_match[4]
            .parse::<i32>()
            .map_err(|e| e.to_string())?,
    );
    vals.insert(
        vars[2].clone(),
        var_decl_match[6]
            .parse::<i32>()
            .map_err(|e| e.to_string())?,
    );
    vals.insert(
        vars[3].clone(),
        var_decl_match[8]
            .parse::<i32>()
            .map_err(|e| e.to_string())?,
    );

    let h1_regex = Regex::new(r"function\s+([a-zA-Z0-9_$]+)\s*\([^)]*\)\s*\{[^}]*this\.v\s*=")
        .map_err(|e| e.to_string())?;
    let h1_name = h1_regex
        .captures(script_text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "Cannot find fnHelper in instrumentation".to_string())?;

    let dom_regex = Regex::new(r"function\s+([a-zA-Z0-9_$]+)\s*\([^)]*\)\s*\{[^}]*document\.createElement")
        .map_err(|e| e.to_string())?;
    let dom_name = dom_regex
        .captures(script_text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "Cannot find domHelper in instrumentation".to_string())?;

    let ua_pattern = format!(
        r"{}\s*=\s*{}\s*\^\s*\(navigator\.userAgent\s*\?\s*(\d+)\s*:\s*(\d+)\);",
        regex::escape(&vars[0]),
        regex::escape(&vars[0])
    );
    let ua_regex = Regex::new(&ua_pattern).map_err(|e| e.to_string())?;
    let ua_match = ua_regex
        .captures(script_text)
        .ok_or_else(|| "Cannot find userAgent check in instrumentation".to_string())?;
    let ua_key = ua_match[1].parse::<i32>().map_err(|e| e.to_string())?;
    let v0 = vals.get_mut(&vars[0]).unwrap();
    *v0 ^= ua_key;

    let ua_match_range = ua_match.get(0).unwrap().range();
    let ua_end = ua_match_range.end;

    let salt_pattern = format!(
        r"{}=\(\({}\^(\d+)\)&0x7FFFFFFF\)%900000\+100000;",
        regex::escape(&vars[0]),
        regex::escape(&vars[0])
    );
    let salt_regex = Regex::new(&salt_pattern).map_err(|e| e.to_string())?;
    let salt_match = salt_regex
        .find(&script_text[ua_end..])
        .ok_or_else(|| "Cannot find salt start in instrumentation".to_string())?;
    let salt_start = ua_end + salt_match.start();

    let ops_segment = &script_text[ua_end..salt_start];
    let ops: Vec<&str> = ops_segment
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let op_not_and_re =
        Regex::new(r"^([a-zA-Z0-9_$]+)\s*=\s*~\(([a-zA-Z0-9_$]+)\s*&\s*([a-zA-Z0-9_$]+)\)$")
            .unwrap();
    let op_xor_re =
        Regex::new(r"^([a-zA-Z0-9_$]+)\s*=\s*([a-zA-Z0-9_$]+)\s*\^\s*([a-zA-Z0-9_$]+)$")
            .unwrap();
    let op_or_re =
        Regex::new(r"^([a-zA-Z0-9_$]+)\s*=\s*([a-zA-Z0-9_$]+)\s*\|\s*([a-zA-Z0-9_$]+)$")
            .unwrap();
    let op_and_re =
        Regex::new(r"^([a-zA-Z0-9_$]+)\s*=\s*([a-zA-Z0-9_$]+)\s*&\s*([a-zA-Z0-9_$]+)$")
            .unwrap();
    let op_call3_re = Regex::new(
        r"^([a-zA-Z0-9_$]+)\s*=\s*([a-zA-Z0-9_$]+)\(([a-zA-Z0-9_$]+),\s*([a-zA-Z0-9_$]+),\s*([a-zA-Z0-9_$]+)\)$",
    )
    .unwrap();

    for op in ops {
        if let Some(caps) = op_not_and_re.captures(op) {
            let d = &caps[1];
            let s1 = &caps[2];
            let s2 = &caps[3];
            if d == s1 {
                let val_d = *vals.get(d).unwrap_or(&0);
                let val_s2 = *vals.get(s2).unwrap_or(&0);
                vals.insert(d.to_string(), !(val_d & val_s2));
                continue;
            }
        }

        if let Some(caps) = op_xor_re.captures(op) {
            let d = &caps[1];
            let s1 = &caps[2];
            let s2 = &caps[3];
            if d == s1 {
                let val_d = *vals.get(d).unwrap_or(&0);
                let val_s2 = *vals.get(s2).unwrap_or(&0);
                vals.insert(d.to_string(), val_d ^ val_s2);
                continue;
            }
        }

        if let Some(caps) = op_or_re.captures(op) {
            let d = &caps[1];
            let s1 = &caps[2];
            let s2 = &caps[3];
            if d == s1 {
                let val_d = *vals.get(d).unwrap_or(&0);
                let val_s2 = *vals.get(s2).unwrap_or(&0);
                vals.insert(d.to_string(), val_d | val_s2);
                continue;
            }
        }

        if let Some(caps) = op_and_re.captures(op) {
            let d = &caps[1];
            let s1 = &caps[2];
            let s2 = &caps[3];
            if d == s1 {
                let val_d = *vals.get(d).unwrap_or(&0);
                let val_s2 = *vals.get(s2).unwrap_or(&0);
                vals.insert(d.to_string(), val_d & val_s2);
                continue;
            }
        }

        if let Some(caps) = op_call3_re.captures(op) {
            let d = &caps[1];
            let fn_name = &caps[2];
            let s1 = &caps[3];
            let s2 = &caps[4];
            let s3 = &caps[5];

            if fn_name == h1_name && d == s3 {
                let val_s1 = *vals.get(s1).unwrap_or(&0);
                let val_s2 = *vals.get(s2).unwrap_or(&0);
                let val_d = *vals.get(d).unwrap_or(&0);
                vals.insert(d.to_string(), fn_helper(val_s1, val_s2, val_d));
                continue;
            }

            if fn_name == dom_name {
                let val_s1 = *vals.get(s1).unwrap_or(&0);
                let val_s2 = *vals.get(s2).unwrap_or(&0);
                let val_s3 = *vals.get(s3).unwrap_or(&0);
                vals.insert(d.to_string(), dom_sum_mock(val_s1, val_s2, val_s3));
                continue;
            }
        }
    }

    for var in &vars {
        let pattern = format!(
            r"{}=\(\({}\^(\d+)\)&0x7FFFFFFF\)%900000\+100000;",
            regex::escape(var),
            regex::escape(var)
        );
        let s_regex = Regex::new(&pattern).map_err(|e| e.to_string())?;
        let s_val = s_regex
            .captures(script_text)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<i32>().ok())
            .ok_or_else(|| format!("Cannot find salt for {var}"))?;
        let curr = *vals.get(var).unwrap_or(&0);
        let final_val = (((curr ^ s_val) & 0x7FFFFFFF) % 900000) + 100000;
        vals.insert(var.clone(), final_val);
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Ok(CapInstrumentationResult {
        nonce,
        state: vals,
        timestamp,
    })
}

/// Fetch, solve, and redeem challenge from Cap endpoint
pub fn solve_challenge(endpoint: &str) -> Result<String, String> {
    let endpoint = endpoint.trim_end_matches('/');
    let challenge_url = format!("{endpoint}/challenge");

    let resp_res = ureq::post(&challenge_url)
        .timeout(Duration::from_secs(10))
        .set("Accept", "application/json")
        .set("Origin", "https://lastudio.cc")
        .set("Referer", "https://lastudio.cc/")
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .call();

    let body_str = match resp_res {
        Ok(resp) => resp
            .into_string()
            .map_err(|e| format!("读取挑战响应失败: {e}"))?,
        Err(ureq::Error::Status(code, resp)) => {
            let err_body = resp.into_string().unwrap_or_default();
            return Err(format!("获取挑战失败 HTTP {code}: {err_body}"));
        }
        Err(e) => return Err(format!("连接挑战端点失败: {e}")),
    };

    let challenge_json: Value =
        serde_json::from_str(&body_str).map_err(|e| format!("解析挑战响应 JSON 失败: {e}"))?;

    if let Some(err) = challenge_json.get("error").and_then(|v| v.as_str()) {
        return Err(format!("挑战服务端返回错误: {err}"));
    }

    let token = challenge_json
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "挑战响应缺少 token".to_string())?;

    let challenge_obj = challenge_json
        .get("challenge")
        .ok_or_else(|| "挑战响应缺少 challenge 节点".to_string())?;

    let count = challenge_obj
        .get("c")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "缺少 count (c)".to_string())? as usize;

    let salt_len = challenge_obj
        .get("s")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "缺少 salt_len (s)".to_string())? as usize;

    let target_len = challenge_obj
        .get("d")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "缺少 target_len (d)".to_string())? as usize;

    let instrumentation_str = challenge_json
        .get("instrumentation")
        .and_then(|v| v.as_str());

    // 求解 PoW
    let solutions = solve_pow(token, count, salt_len, target_len);
    let solutions_num: Vec<u64> = solutions
        .iter()
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();

    // 求解 Instrumentation（若存在）
    let instr_result = if let Some(instr_b64) = instrumentation_str {
        if !instr_b64.trim().is_empty() {
            let compressed = decode_base64_instrumentation(instr_b64)?;
            let script_text = decompress_raw_deflate(&compressed)?;
            Some(solve_instrumentation(&script_text)?)
        } else {
            None
        }
    } else {
        None
    };

    // 提交赎回兑换验证凭据
    let redeem_url = format!("{endpoint}/redeem");
    let mut redeem_body = json!({
        "token": token,
        "solutions": solutions_num,
    });

    if let Some(instr) = instr_result
        && let Some(obj) = redeem_body.as_object_mut()
    {
        obj.insert(
            "instr".to_string(),
            json!({
                "i": instr.nonce,
                "state": instr.state,
                "ts": instr.timestamp,
            }),
        );
    }

    let redeem_res = ureq::post(&redeem_url)
        .timeout(Duration::from_secs(10))
        .set("Accept", "application/json")
        .set("Content-Type", "application/json")
        .set("Origin", "https://lastudio.cc")
        .set("Referer", "https://lastudio.cc/")
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .send_json(&redeem_body);

    let redeem_str = match redeem_res {
        Ok(resp) => resp
            .into_string()
            .map_err(|e| format!("读取赎回响应失败: {e}"))?,
        Err(ureq::Error::Status(code, resp)) => {
            let err_body = resp.into_string().unwrap_or_default();
            return Err(format!("验证赎回失败 HTTP {code}: {err_body}"));
        }
        Err(e) => return Err(format!("连接赎回端点失败: {e}")),
    };

    let redeem_json: Value =
        serde_json::from_str(&redeem_str).map_err(|e| format!("解析赎回响应 JSON 失败: {e}"))?;

    let success = redeem_json
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        let err = redeem_json
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("验证未通过，请重试");
        return Err(err.to_string());
    }

    let solved_token = redeem_json
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "赎回响应缺少 token".to_string())?
        .to_string();

    Ok(solved_token)
}
