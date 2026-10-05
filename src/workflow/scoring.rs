use serde::{Deserialize, Serialize};
use serde_json::json;

pub const GRADUATION: f64 = 0.125;
pub const LOSS_PROPENSITY_EXPONENT: f64 = 0.85;
pub const LOSS_MISTAKE_EXPONENT: f64 = 1.80;
pub const BIG_QUESTION_SHARE: f64 = 0.15;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScoreProfile {
    pub total: f64,
    pub accuracy: f64,
    pub fluency: f64,
    pub integrity: f64,
    pub stress: f64,
}

impl Default for ScoreProfile {
    fn default() -> Self {
        Self {
            total: 5.0,
            accuracy: 4.95,
            fluency: 4.95,
            integrity: 4.95,
            stress: 4.90,
        }
    }
}

impl ScoreProfile {
    pub fn new(total: f64) -> Self {
        let t = total.clamp(0.0, 5.0);
        let factor = t / 5.0;
        Self {
            total: t,
            accuracy: (4.95 * factor).clamp(0.0, 5.0),
            fluency: (4.95 * factor).clamp(0.0, 5.0),
            integrity: (4.95 * factor).clamp(0.0, 5.0),
            stress: (4.90 * factor).clamp(0.0, 5.0),
        }
    }

    pub fn clamped(&self) -> Self {
        let clamp = |v: f64| v.clamp(0.0, 5.0);
        Self {
            total: clamp(self.total),
            accuracy: clamp(self.accuracy),
            fluency: clamp(self.fluency),
            integrity: clamp(self.integrity),
            stress: clamp(self.stress),
        }
    }
}

/// 真实标准四舍五入（避免 Python/Rust 银行家舍入误差）
pub fn round_points(value: f64) -> f64 {
    (value * 100.0 + 0.5).floor() / 100.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionAllocation {
    pub order: usize,
    pub is_objective: bool,
    pub type_name: String,
    pub max_score: f64,
    pub allocated_score: f64,
    pub ise_score: f64,
    pub is_full: bool,
    pub is_mistake: bool,
    pub is_big_question: bool,
}

#[derive(Debug, Clone)]
pub struct UseTimeBreakdown {
    pub prep_seconds: u32,
    pub listen_seconds: u32,
    pub answer_seconds: u32,
    pub overhead_seconds: u32,
    pub total_seconds: u32,
}

/// 极轻量 XorShift64 伪随机数发生器
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0x8a5b_4c32_1e9f_a0b1
        } else {
            seed
        };
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    fn next_gaussian(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-15);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }

    fn next_lognormal(&mut self, mean: f64, sigma: f64) -> f64 {
        if sigma <= 0.0 {
            mean.exp()
        } else {
            (mean + sigma * self.next_gaussian()).exp()
        }
    }
}

pub fn is_objective_question(q: &crate::models::ParsedQuestion) -> bool {
    let cat = q.category.to_lowercase();
    let display = q.display_category().to_lowercase();
    let prompt = q.prompt_text.to_lowercase();

    let matches = |s: &str| {
        s.contains("choose")
            || s.contains("选择")
            || s.contains("select")
            || s.contains("option")
            || s.contains("单选")
            || s.contains("多选")
            || s.contains("判断")
            || s.contains("choice")
    };

    matches(&cat) || matches(&display) || matches(&prompt) || !q.options.is_empty()
}

fn is_read_like_category(cat: &str, display: &str) -> bool {
    cat.contains("read")
        || display.contains("朗读")
        || cat.contains("repeat")
        || display.contains("复述")
        || cat.contains("picture")
        || display.contains("看图")
        || cat.contains("essay")
        || display.contains("表达")
}

fn count_words(text: &str) -> usize {
    let raw = text.trim();
    if raw.is_empty() {
        return 0;
    }
    raw.split_whitespace().count()
}

fn sample_weighted(rng: &mut XorShift64, weights: &[f64], count: usize) -> Vec<usize> {
    let mut pool: Vec<usize> = (0..weights.len()).collect();
    let mut picked = Vec::new();
    let work: Vec<f64> = weights.to_vec();

    for _ in 0..count.min(pool.len()) {
        let total: f64 = pool.iter().map(|&i| work[i]).sum();
        if total <= 0.0 {
            if !pool.is_empty() {
                picked.push(pool.remove(0));
            }
            continue;
        }
        let threshold = rng.next_f64() * total;
        let mut running = 0.0;
        let mut chosen = *pool.last().unwrap();
        for &idx in &pool {
            running += work[idx];
            if running >= threshold {
                chosen = idx;
                break;
            }
        }
        picked.push(chosen);
        pool.retain(|&i| i != chosen);
    }
    picked
}

/// 加权随机拟真控分规划
pub fn plan_question_allocations(
    questions: &[crate::models::ParsedQuestion],
    target_total: f64,
    seed: u64,
    allow_over_max: bool,
) -> Vec<QuestionAllocation> {
    let n = questions.len();
    if n == 0 {
        return Vec::new();
    }

    let mut rng = XorShift64::new(seed);
    let target = target_total.max(0.0);

    // 确定考卷标准总分
    let paper_total = if target <= 10.0 && n <= 10 {
        10.0
    } else if target <= 20.0 {
        20.0
    } else if target <= 30.0 {
        30.0
    } else if target <= 40.0 {
        40.0
    } else if target <= 50.0 {
        50.0
    } else if target <= 100.0 {
        100.0
    } else if target <= 120.0 {
        120.0
    } else if target <= 150.0 {
        150.0
    } else {
        target.ceil()
    };

    // 计算各题相对权重
    let mut raw_weights = Vec::with_capacity(n);
    for q in questions {
        let is_obj = is_objective_question(q);
        let cat = q.category.to_lowercase();
        let disp = q.display_category().to_lowercase();
        let words = count_words(&q.prompt_text).max(
            q.answers.first().map(|a| count_words(a)).unwrap_or(0),
        );

        let w = if is_read_like_category(&cat, &disp) {
            if words >= 40 {
                5.0
            } else if words >= 15 {
                3.0
            } else {
                2.0
            }
        } else if cat.contains("role") || cat.contains("dialogue") || cat.contains("3q5a") || cat.contains("fill") {
            2.0
        } else if is_obj {
            1.5
        } else {
            1.0
        };
        raw_weights.push(w);
    }

    let total_w: f64 = raw_weights.iter().sum();
    let scale = if total_w > 0.0 {
        paper_total / total_w
    } else {
        paper_total / (n as f64)
    };

    // 0.125 档位量化：以 GRADUATION = 0.125 为步长计算 max_slots
    let total_max_slots = (paper_total / GRADUATION).round() as i32;
    let mut max_slots = Vec::with_capacity(n);
    let mut slot_remainders = Vec::with_capacity(n);

    for (i, &w) in raw_weights.iter().enumerate() {
        let ideal_val = w * scale;
        let ideal_slots = ideal_val / GRADUATION;
        let base_slots = ideal_slots.floor() as i32;
        let rem = ideal_slots - base_slots as f64;
        max_slots.push(base_slots.max(1));
        slot_remainders.push((i, rem));
    }

    // 最大余数法配平 max_slots
    let mut diff = total_max_slots - max_slots.iter().sum::<i32>();
    slot_remainders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut cursor = 0;
    while diff > 0 && cursor < slot_remainders.len() {
        let idx = slot_remainders[cursor].0;
        max_slots[idx] += 1;
        diff -= 1;
        cursor += 1;
    }
    while diff < 0 && cursor < slot_remainders.len() {
        let idx = slot_remainders[cursor].0;
        if max_slots[idx] > 1 {
            max_slots[idx] -= 1;
            diff += 1;
        }
        cursor += 1;
    }

    let mut full_points: Vec<f64> = max_slots
        .iter()
        .map(|&s| round_points(s as f64 * GRADUATION))
        .collect();
    let current_full_sum: f64 = full_points.iter().sum();
    let full_diff = round_points(paper_total - current_full_sum);
    if full_diff.abs() > 1e-4 && !full_points.is_empty() {
        full_points[n - 1] = round_points(full_points[n - 1] + full_diff);
        max_slots[n - 1] = (full_points[n - 1] / GRADUATION).round() as i32;
    }

    // 占比 >= 15% 的长句大题避免绝对满分
    let share: Vec<f64> = full_points.iter().map(|&p| p / paper_total).collect();
    let is_big_question: Vec<bool> = (0..n)
        .map(|i| share[i] >= BIG_QUESTION_SHARE && !is_objective_question(&questions[i]))
        .collect();

    // 超满分特殊处理 (allow_over_max = true 且 target > paper_total)
    if allow_over_max && target > paper_total {
        let target_slots = (target / GRADUATION).round() as i32;
        let extra_slots = target_slots - total_max_slots;
        let mut allocated_slots = max_slots.clone();
        for i in 0..extra_slots as usize {
            allocated_slots[i % n] += 1;
        }
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            let allocated_score = round_points(allocated_slots[i] as f64 * GRADUATION);
            let raw_ise = if full_points[i] > 0.0 {
                round_points(5.0 * allocated_score / full_points[i])
            } else {
                5.0
            };
            result.push(QuestionAllocation {
                order: i + 1,
                is_objective: is_objective_question(&questions[i]),
                type_name: questions[i].display_category().to_string(),
                max_score: full_points[i],
                allocated_score,
                ise_score: raw_ise,
                is_full: true,
                is_mistake: false,
                is_big_question: is_big_question[i],
            });
        }
        return result;
    }

    // 目标档位与扣分档位
    let target_slots = ((target / GRADUATION).round() as i32).clamp(0, total_max_slots);
    let total_loss_slots = total_max_slots - target_slots;

    if total_loss_slots <= 0 {
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            result.push(QuestionAllocation {
                order: i + 1,
                is_objective: is_objective_question(&questions[i]),
                type_name: questions[i].display_category().to_string(),
                max_score: full_points[i],
                allocated_score: full_points[i],
                ise_score: 5.0,
                is_full: true,
                is_mistake: false,
                is_big_question: is_big_question[i],
            });
        }
        return result;
    }

    // 主客观隔离
    let mut obj_indices = Vec::new();
    let mut subj_indices = Vec::new();
    for (i, q) in questions.iter().enumerate() {
        if is_objective_question(q) {
            obj_indices.push(i);
        } else {
            subj_indices.push(i);
        }
    }

    let mut is_mistake = vec![false; n];
    let mut allocated_slots = vec![0i32; n];

    // 客观题二值化：仅给 0 或 满分
    let mut obj_awarded_sum = 0.0;
    if !obj_indices.is_empty() {
        let subj_full: f64 = subj_indices.iter().map(|&i| full_points[i]).sum();
        let obj_floor = (target - subj_full).max(0.0);

        let mut order = obj_indices.clone();
        order.sort_by(|&a, &b| {
            let wa = -(full_points[a].max(0.0).powf(0.9)) * (0.5 + rng.next_f64());
            let wb = -(full_points[b].max(0.0).powf(0.9)) * (0.5 + rng.next_f64());
            wa.partial_cmp(&wb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut obj_full_set = std::collections::HashSet::new();
        for &idx in &order {
            let pt = full_points[idx];
            if obj_awarded_sum + pt <= target + 1e-9 {
                obj_full_set.insert(idx);
                obj_awarded_sum += pt;
            }
        }
        for &idx in &order {
            if obj_awarded_sum >= obj_floor - 1e-9 {
                break;
            }
            if !obj_full_set.contains(&idx) {
                obj_full_set.insert(idx);
                obj_awarded_sum += full_points[idx];
            }
        }
        for &idx in order.iter().rev() {
            if obj_awarded_sum <= target + 1e-9 {
                break;
            }
            if obj_full_set.contains(&idx) {
                obj_full_set.remove(&idx);
                obj_awarded_sum -= full_points[idx];
            }
        }

        for &idx in &obj_indices {
            if obj_full_set.contains(&idx) {
                allocated_slots[idx] = max_slots[idx];
                is_mistake[idx] = false;
            } else {
                allocated_slots[idx] = 0;
                is_mistake[idx] = true;
            }
        }
    }

    // 主观题扣分档位加权随机拟真分配
    if !subj_indices.is_empty() {
        let subj_count = subj_indices.len();
        let subj_full_slots: i32 = subj_indices.iter().map(|&i| max_slots[i]).sum();
        let subj_target = (target - obj_awarded_sum).clamp(0.0, paper_total);
        let subj_target_slots = ((subj_target / GRADUATION).round() as i32).clamp(0, subj_full_slots);
        let subj_loss_slots = subj_full_slots - subj_target_slots;

        if subj_loss_slots <= 0 {
            for &idx in &subj_indices {
                allocated_slots[idx] = max_slots[idx];
            }
        } else {
            let subj_points: Vec<f64> = subj_indices.iter().map(|&i| full_points[i]).collect();
            let subj_m_slots: Vec<i32> = subj_indices.iter().map(|&i| max_slots[i]).collect();
            let total_subj_full: f64 = subj_points.iter().sum::<f64>().max(1e-6);

            let subj_shares: Vec<f64> = subj_points.iter().map(|&p| p / total_subj_full).collect();
            // 扣分倾向权重 P_i = S_i^0.85
            let propensity: Vec<f64> = subj_shares
                .iter()
                .map(|&s| s.max(1e-6).powf(LOSS_PROPENSITY_EXPONENT))
                .collect();
            // 失误题概率 B_i = S_i^1.80
            let mistake_bias: Vec<f64> = subj_shares
                .iter()
                .map(|&s| s.max(1e-6).powf(LOSS_MISTAKE_EXPONENT))
                .collect();

            let deduction_ratio = subj_loss_slots as f64 / (subj_full_slots as f64).max(1.0);
            let sigma = 0.26 + 0.45 * deduction_ratio;

            let avg_max_slots = subj_full_slots as f64 / subj_count as f64;
            let raw_mistake_cnt = 1.0 + (subj_loss_slots as f64 / avg_max_slots.max(1.0)) * 1.55;
            let mistake_count = (raw_mistake_cnt.round() as usize)
                .max(1)
                .min(subj_count)
                .min((subj_count as f64 * 0.6).round() as usize)
                .max(1);

            let big_subj_indices: Vec<usize> = (0..subj_count)
                .filter(|&i| is_big_question[subj_indices[i]])
                .collect();

            let mut mistakes = std::collections::HashSet::new();
            if !big_subj_indices.is_empty() && deduction_ratio <= 0.40 && subj_loss_slots >= big_subj_indices.len() as i32 {
                for &b_idx in &big_subj_indices {
                    mistakes.insert(b_idx);
                }
                let extra = mistake_count.saturating_sub(mistakes.len());
                let others: Vec<usize> = (0..subj_count).filter(|i| !mistakes.contains(i)).collect();
                if extra > 0 && !others.is_empty() {
                    let weights: Vec<f64> = others.iter().map(|&i| mistake_bias[i]).collect();
                    let picked = sample_weighted(&mut rng, &weights, extra);
                    for p in picked {
                        mistakes.insert(others[p]);
                    }
                }
            } else {
                let picked = sample_weighted(&mut rng, &mistake_bias, mistake_count);
                for p in picked {
                    mistakes.insert(p);
                }
            }

            // 失误题承担 70%~90% 扣分
            let mistake_share = if deduction_ratio <= 0.15 {
                0.90
            } else if deduction_ratio <= 0.30 {
                0.82
            } else {
                0.70
            };

            let mistake_weight: f64 = mistakes.iter().map(|&i| propensity[i]).sum();
            let mistake_weight = if mistake_weight <= 0.0 { 1.0 } else { mistake_weight };
            let ordinary_count = subj_count - mistakes.len();

            let mut ideal = vec![0.0; subj_count];
            for i in 0..subj_count {
                let base = if mistakes.contains(&i) {
                    subj_loss_slots as f64 * mistake_share * propensity[i] / mistake_weight
                } else if ordinary_count > 0 {
                    subj_loss_slots as f64 * (1.0 - mistake_share) / ordinary_count as f64
                } else {
                    0.0
                };
                let jitter = rng.next_lognormal(0.0, sigma);
                ideal[i] = base * jitter;
            }

            let ideal_sum: f64 = ideal.iter().sum();
            if ideal_sum > 0.0 {
                let rescale = subj_loss_slots as f64 / ideal_sum;
                for v in &mut ideal {
                    *v *= rescale;
                }
            }

            let target_rate = (1.0 - deduction_ratio) * 100.0;
            let cap_ratio = if target_rate >= 93.0 {
                0.25
            } else if target_rate >= 88.0 {
                0.35
            } else if target_rate >= 78.0 {
                0.50
            } else {
                0.70
            };

            let headroom: Vec<i32> = subj_m_slots.iter().map(|&s| (s - 1).max(0)).collect();
            let mut caps: Vec<i32> = (0..subj_count)
                .map(|i| ((subj_m_slots[i] as f64 * cap_ratio).floor() as i32).min(headroom[i]).max(0))
                .collect();

            let mut loss = vec![0i32; subj_count];
            for i in 0..subj_count {
                let mut base = ideal[i].floor() as i32;
                if rng.next_f64() < (ideal[i] - base as f64) {
                    base += 1;
                }
                loss[i] = base.clamp(0, caps[i]);
            }

            // 最大余数法配平残差，确保 sum(allocated) ≡ target_total
            let mut remaining = subj_loss_slots - loss.iter().sum::<i32>();
            let budget = 2 * remaining.abs() + 8 * (subj_count as i32) + 64;
            let mut guard = 0;
            while remaining != 0 {
                guard += 1;
                if guard > budget {
                    break;
                }
                let mut candidates = Vec::new();
                for i in 0..subj_count {
                    if remaining > 0 {
                        if loss[i] < caps[i] {
                            candidates.push((i, (ideal[i] - loss[i] as f64 + 0.5).max(0.05)));
                        }
                    } else if loss[i] > 0 {
                        candidates.push((i, (loss[i] as f64 - ideal[i] + 0.5).max(0.05)));
                    }
                }
                if candidates.is_empty() {
                    let mut relaxed = false;
                    for i in 0..subj_count {
                        if caps[i] < headroom[i] {
                            caps[i] = headroom[i];
                            relaxed = true;
                        }
                    }
                    if !relaxed {
                        for i in 0..subj_count {
                            if caps[i] < subj_m_slots[i] {
                                caps[i] = subj_m_slots[i];
                                relaxed = true;
                            }
                        }
                    }
                    if !relaxed {
                        break;
                    }
                    continue;
                }
                let total_cand_w: f64 = candidates.iter().map(|(_, w)| *w).sum();
                let threshold = rng.next_f64() * total_cand_w;
                let mut running = 0.0;
                let mut chosen = candidates.last().unwrap().0;
                for (idx, w) in &candidates {
                    running += *w;
                    if running >= threshold {
                        chosen = *idx;
                        break;
                    }
                }
                if remaining > 0 {
                    loss[chosen] += 1;
                    remaining -= 1;
                } else {
                    loss[chosen] -= 1;
                    remaining += 1;
                }
            }

            // 占比 >= 15% 的长句大题避免绝对满分（至少扣 1 档 0.125）
            let big: Vec<usize> = (0..subj_count)
                .filter(|&i| is_big_question[subj_indices[i]] && caps[i] >= 1)
                .collect();
            if !big.is_empty() && subj_loss_slots >= big.len() as i32 {
                for &index in &big {
                    if loss[index] > 0 {
                        continue;
                    }
                    let donors: Vec<usize> = (0..subj_count)
                        .filter(|&j| j != index && (loss[j] >= 2 || (loss[j] >= 1 && !is_big_question[subj_indices[j]])))
                        .collect();
                    if let Some(&donor) = donors.iter().max_by_key(|&&j| loss[j]) {
                        loss[donor] -= 1;
                        loss[index] += 1;
                    }
                }
            }

            for (i, &subj_idx) in subj_indices.iter().enumerate() {
                allocated_slots[subj_idx] = (max_slots[subj_idx] - loss[i]).max(0);
                if mistakes.contains(&i) || loss[i] > 1 {
                    is_mistake[subj_idx] = true;
                }
            }
        }
    }

    // 构造分配明细并反推科大讯飞评测 ise_score
    let mut allocations = Vec::with_capacity(n);
    for i in 0..n {
        let max_sc = full_points[i];
        let mut alloc_sc = (allocated_slots[i] as f64 * GRADUATION).min(max_sc);
        alloc_sc = round_points(alloc_sc);

        // 反推科大讯飞评测 ise_score（两位小数），保证官方 ceil 向上取整后精确还原 allocated_score
        let raw_ise = if max_sc <= 0.0 {
            5.0
        } else {
            let raw = (alloc_sc * 5.0 / max_sc) - 0.0001;
            if raw >= 5.0 {
                5.0
            } else {
                (raw * 100.0).floor().max(0.0) / 100.0
            }
        };

        allocations.push(QuestionAllocation {
            order: i + 1,
            is_objective: is_objective_question(&questions[i]),
            type_name: questions[i].display_category().to_string(),
            max_score: max_sc,
            allocated_score: alloc_sc,
            ise_score: raw_ise,
            is_full: (alloc_sc - max_sc).abs() < 1e-4,
            is_mistake: is_mistake[i],
            is_big_question: is_big_question[i],
        });
    }

    // 最大余数法配平残差，确保 sum(allocated_score) ≡ target_total
    let current_sum: f64 = allocations.iter().map(|a| a.allocated_score).sum();
    let mut residual = round_points(target - current_sum);

    if residual.abs() > 1e-4 {
        let mut cand_indices: Vec<usize> = (0..n).filter(|&i| !allocations[i].is_objective).collect();
        if cand_indices.is_empty() {
            cand_indices = (0..n).collect();
        }
        for &idx in &cand_indices {
            if residual.abs() < 1e-4 {
                break;
            }
            if residual > 0.0 && allocations[idx].allocated_score < allocations[idx].max_score {
                let add = residual.min(round_points(allocations[idx].max_score - allocations[idx].allocated_score));
                allocations[idx].allocated_score = round_points(allocations[idx].allocated_score + add);
                residual = round_points(residual - add);
            } else if residual < 0.0 && allocations[idx].allocated_score > 0.0 {
                let sub = (-residual).min(allocations[idx].allocated_score);
                allocations[idx].allocated_score = round_points(allocations[idx].allocated_score - sub);
                residual = round_points(residual + sub);
            }
            let max_sc = allocations[idx].max_score;
            let alloc_sc = allocations[idx].allocated_score;
            allocations[idx].ise_score = if max_sc <= 0.0 {
                5.0
            } else {
                let raw = (alloc_sc * 5.0 / max_sc) - 0.0001;
                if raw >= 5.0 {
                    5.0
                } else {
                    (raw * 100.0).floor().max(0.0) / 100.0
                }
            };
            allocations[idx].is_full = (alloc_sc - max_sc).abs() < 1e-4;
        }
    }

    allocations
}

/// 智能作答耗时计算与秒级哈希抖动
pub fn calculate_smart_use_time(homework: &crate::models::HomeworkItem, nonce: u64) -> (u32, UseTimeBreakdown) {
    let mut prep_sec: u32 = 0;
    let mut listen_sec: u32 = 0;
    let mut answer_sec: u32 = 0;
    let mut overhead_sec: u32 = 0;

    let sec_count = homework.sections.len() as u32;

    if sec_count > 0 {
        // 大题引导 15s
        overhead_sec += sec_count * 15;

        for sec in &homework.sections {
            let is_read_sec = sec.title.contains("朗读")
                || sec.section_type.contains("read")
                || sec.title.to_lowercase().contains("read");
            let is_double_sec = sec.title.contains("听选")
                || sec.title.contains("转述")
                || sec.title.contains("复述")
                || sec.section_type.contains("listen_choose")
                || sec.section_type.contains("repeat");

            for (q_idx, q) in sec.questions.iter().enumerate() {
                // 小题缓冲 5s
                overhead_sec += 5;

                let is_obj = is_objective_question(q);
                let cat_lower = q.category.to_lowercase();
                let is_double_q = is_double_sec
                    || cat_lower.contains("听选")
                    || cat_lower.contains("转述")
                    || cat_lower.contains("复述")
                    || cat_lower.contains("repeat")
                    || cat_lower.contains("listen_choose");

                // 准备时长：朗读大题首题 50s，其余口语题 8s
                if is_read_sec && q_idx == 0 {
                    prep_sec += 50;
                } else if !is_obj {
                    prep_sec += 8;
                }

                // 听题时长：若含两遍题型（听选信息、转述等）乘 2
                let is_listen_q = is_double_q
                    || sec.title.contains("听")
                    || cat_lower.contains("listen")
                    || is_obj
                    || cat_lower.contains("dialogue");
                if is_listen_q {
                    let prompt_w = count_words(&q.prompt_text);
                    let mut q_listen = (prompt_w as f64 / 2.2).max(5.0);
                    if is_double_q {
                        q_listen *= 2.0;
                    }
                    listen_sec += q_listen.round() as u32;
                }

                // 作答时长：单词数 / 2.2
                if is_obj {
                    answer_sec += 4;
                } else {
                    let ans_text = if !q.answers.is_empty() {
                        q.answers.join(" ")
                    } else if !q.prompt_text.is_empty() {
                        q.prompt_text.clone()
                    } else {
                        String::new()
                    };
                    let words = count_words(&ans_text);
                    let q_ans = if words > 0 {
                        (words as f64 / 2.2).max(3.0)
                    } else {
                        5.0
                    };
                    answer_sec += q_ans.round() as u32;
                }
            }
        }
    } else {
        let q_cnt = homework.question_count.max(1) as u32;
        overhead_sec = 15 + q_cnt * 5;
        prep_sec = 50 + q_cnt.saturating_sub(1) * 8;
        listen_sec = q_cnt * 6;
        answer_sec = q_cnt * 5;
    }

    let base_total = prep_sec + listen_sec + answer_sec + overhead_sec;

    // 散列秒数抖动：使用作业 ID + nonce 的哈希生成 ±12% 抖动，结果四舍五入到具体秒数
    let mut h = 0xcbf29ce484222325u64;
    for b in homework.id.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h ^= nonce;
    h = h.wrapping_mul(0x100000001b3);

    // [0, 9999] 映射至 [-0.12, +0.12]
    let jitter_frac = -0.12 + ((h % 10000) as f64 / 9999.0) * 0.24;
    let jittered = (base_total as f64 * (1.0 + jitter_frac) + 0.5).floor() as u32;

    let is_mock = homework.name.contains("模拟") || homework.name.contains("听说") || homework.set_type == 1;
    let is_pk = homework.pk == 1 || homework.set_type == 6;

    let final_total = if is_mock {
        jittered.max(180)
    } else if is_pk {
        jittered.max(30)
    } else {
        jittered.max(60)
    };

    let breakdown = UseTimeBreakdown {
        prep_seconds: prep_sec,
        listen_seconds: listen_sec,
        answer_seconds: answer_sec,
        overhead_seconds: overhead_sec,
        total_seconds: final_total,
    };

    (final_total, breakdown)
}

pub fn reroll_time_jitter(homework: &crate::models::HomeworkItem, nonce: u64) -> u32 {
    calculate_smart_use_time(homework, nonce).0
}

/// 估算拟真做题用时（秒）- 兼容旧版调用
pub fn calculate_recommended_use_time(homework: &crate::models::HomeworkItem) -> u32 {
    let mut sum_duration = 0.0;
    for section in &homework.sections {
        for q in &section.questions {
            let dur = if q.category.contains("choose") {
                6.0
            } else {
                let text = if !q.prompt_text.trim().is_empty() {
                    q.prompt_text.trim()
                } else if let Some(first_ans) = q.answers.first() {
                    first_ans.trim()
                } else {
                    ""
                };
                if !text.is_empty() {
                    let words = text.split_whitespace().count();
                    if words > 0 {
                        (words as f64 / 2.2).max(3.0)
                    } else {
                        5.0
                    }
                } else {
                    5.0
                }
            };
            sum_duration += dur;
        }
    }

    if homework.sections.is_empty() && homework.question_count > 0 {
        sum_duration = homework.question_count as f64 * 6.0;
    }

    let recommended = (sum_duration * 2.0 + 120.0).round() as u32;

    let is_mock = homework.name.contains("模拟") || homework.name.contains("听说") || homework.set_type == 1;
    let is_pk = homework.pk == 1 || homework.set_type == 6;

    if is_mock {
        recommended.max(180)
    } else if is_pk {
        recommended.max(30)
    } else {
        recommended.max(60)
    }
}

pub fn convert_score(score: f64, maximum: f64, graduation: f64, cap: bool) -> f64 {
    if maximum <= 0.0 {
        return 0.0;
    }
    let mut value = score * maximum / 5.0;
    if graduation > 0.0 {
        value = ((value - 1e-12) / graduation).ceil() * graduation;
    }
    if cap {
        round_points(value.min(maximum))
    } else {
        round_points(value)
    }
}

#[inline(never)]
pub fn build_score_detail(
    profile: &ScoreProfile,
    category: &str,
    choose: &str,
    repeat_mode: bool,
    graduation: f64,
) -> String {
    crate::themida_tiger_red!({
        let p = profile.clamped();
        let detail = json!({
            "total_score": format!("{:.2}", p.total),
            "choose": choose,
            "fluency_score": format!("{:.2}", p.fluency),
            "accuracy_score": format!("{:.2}", p.accuracy),
            "integrity_score": format!("{:.2}", p.integrity),
            "standard_score": format!("{:.2}", p.stress),
            "category": category,
            "repeat_mode": if repeat_mode { "0" } else { "1" },
            "version": "3",
            "rate_scale": if (graduation - 0.125).abs() < 1e-6 { 1 } else { 0 }
        });
        serde_json::to_string(&detail).unwrap_or_else(|_| "{}".to_string())
    })
}

fn phonemes(word: &str) -> Vec<String> {
    let clean: String = word.chars().filter(|c| c.is_alphabetic()).collect();
    let lower = clean.to_lowercase();
    if lower.is_empty() {
        return vec!["*".to_string()];
    }

    let pairs = [
        ("th", "θ"), ("sh", "ʃ"), ("ch", "tʃ"), ("wh", "w"), ("ph", "f"), ("ck", "k"),
        ("ng", "ŋ"), ("oo", "uː"), ("ee", "iː"), ("ea", "iː"), ("ou", "aʊ"),
        ("ow", "aʊ"), ("ai", "eɪ"), ("ay", "eɪ"), ("oi", "ɔɪ"), ("oy", "ɔɪ"),
    ];

    let mut result = Vec::new();
    let chars: Vec<char> = lower.chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        if idx + 1 < chars.len() {
            let pair: String = chars[idx..=idx + 1].iter().collect();
            if let Some(&(_, phone)) = pairs.iter().find(|&&(p, _)| p == pair) {
                result.push(phone.to_string());
                idx += 2;
                continue;
            }
        }
        result.push(chars[idx].to_string());
        idx += 1;
    }
    result
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[inline(never)]
pub fn generate_score_xml(
    text: &str,
    profile: &ScoreProfile,
    category: &str,
    seed: u64,
    clamp: bool,
) -> String {
    crate::themida_tiger_red!({
        let p = if clamp { profile.clamped() } else { *profile };
        let category_name = match category {
            "read_sentence" | "read_chapter" | "talk_recite" | "talk_free" => category,
            _ => "read_chapter",
        };

        // 伪随机微调波动（保持平稳逼真，不超出范围）
        let pseudo_noise = ((seed % 17) as f64 - 8.0) * 0.01;
        let standard = if clamp {
            (p.total + pseudo_noise).clamp(0.0, 5.0)
        } else {
            (p.total + pseudo_noise).max(0.0)
        };

        let sentences: Vec<&str> = text
            .split(['.', '!', '?'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let valid_sentences = if sentences.is_empty() {
            vec![text.trim()]
        } else {
            sentences
        };

        let mut xml = String::with_capacity(1024);
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
        xml.push_str("<xml_result>");
        xml.push_str(&format!("<{category_name}>"));
        xml.push_str(&format!(
            "<rec_paper total_score=\"{:.2}\" accuracy_score=\"{:.2}\" fluency_score=\"{:.2}\" integrity_score=\"{:.2}\" standard_score=\"{:.2}\" stress_score=\"{:.2}\" is_rejected=\"false\" except_info=\"\" content=\"{}\">",
            p.total, p.accuracy, p.fluency, p.integrity, standard, p.stress, escape_xml(text)
        ));

        for (s_idx, sentence_str) in valid_sentences.iter().enumerate() {
            let s_noise = (((seed + s_idx as u64 * 7) % 13) as f64 - 6.0) * 0.01;
            let s_score = if clamp {
                (p.total + s_noise).clamp(0.0, 5.0)
            } else {
                (p.total + s_noise).max(0.0)
            };

            xml.push_str(&format!(
                "<sentence content=\"{}\" total_score=\"{:.2}\">",
                escape_xml(sentence_str), s_score
            ));

            let words: Vec<&str> = sentence_str.split_whitespace().collect();
            for (w_idx, word_str) in words.iter().enumerate() {
                let w_noise = (((seed + w_idx as u64 * 11) % 19) as f64 - 9.0) * 0.015;
                let w_score = if clamp {
                    (s_score + w_noise).clamp(0.0, 5.0)
                } else {
                    (s_score + w_noise).max(0.0)
                };

                xml.push_str(&format!(
                    "<word content=\"{}\" total_score=\"{:.2}\" dp_message=\"0\">",
                    escape_xml(word_str), w_score
                ));

                for ph in phonemes(word_str) {
                    xml.push_str(&format!("<phoneme content=\"{}\"/>", escape_xml(&ph)));
                }
                xml.push_str("</word>");
            }
            xml.push_str("</sentence>");
        }

        xml.push_str("</rec_paper>");
        xml.push_str(&format!("</{category_name}>"));
        xml.push_str("</xml_result>");

        xml
    })
}
