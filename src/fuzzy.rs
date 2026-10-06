//! Fuzzy matching for the command palette (and anything else that filters).
//!
//! A query matches a candidate if its characters appear in order,
//! case-insensitively. Among the ways they can line up, the best-scoring one
//! wins, and it's found exactly (small dynamic program), not greedily — so
//! `gp` matches **G**it: **P**ush on the word starts rather than on the `p`
//! inside "Stop".
//!
//! Scoring, per matched character:
//! - +1 for the match;
//! - +8 if it starts a word (start of string, after a space or `-_/.:`, or a
//!   lower→upper camel boundary), +4 more at the very start;
//! - +6 if it directly follows the previous match;
//! - −1 per skipped character between matches (capped at −6), and up to −3
//!   for how late the first match starts;
//! - +10 once if the whole query matches as one contiguous run — typing a
//!   word's beginning should find that word ("dep" → Deploy, not
//!   **De**lete **p**aper).

/// A successful match: its score and the matched character indices (in
/// `char`s, ascending) for highlighting.
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub score: i32,
    pub positions: Vec<usize>,
}

fn is_separator(c: char) -> bool {
    matches!(c, ' ' | '-' | '_' | '/' | '.' | ':' | '\\')
}

fn word_start(chars: &[char], j: usize) -> bool {
    j == 0 || is_separator(chars[j - 1]) || (chars[j - 1].is_lowercase() && chars[j].is_uppercase())
}

/// Score `query` against `candidate`. `None` if it doesn't match. An empty
/// query matches everything with score 0.
pub fn fuzzy_match(query: &str, candidate: &str) -> Option<Match> {
    let q: Vec<char> = query.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_lowercase).collect();
    if q.is_empty() {
        return Some(Match { score: 0, positions: Vec::new() });
    }
    let t: Vec<char> = candidate.chars().collect();
    let lower: Vec<char> = t.iter().map(|c| c.to_lowercase().next().unwrap_or(*c)).collect();
    let (m, n) = (q.len(), t.len());
    if m > n {
        return None;
    }

    const NONE: i32 = i32::MIN / 2;
    // best[i][j]: best score with q[i] matched at t[j]; from[i][j]: where
    // q[i-1] was matched on that best path.
    let mut best = vec![vec![NONE; n]; m];
    let mut from = vec![vec![usize::MAX; n]; m];

    let bonus = |j: usize| -> i32 {
        let mut b = 1;
        if word_start(&t, j) {
            b += 8;
        }
        if j == 0 {
            b += 4;
        }
        b
    };

    for j in 0..n {
        if lower[j] == q[0] {
            best[0][j] = bonus(j) - (j as i32).min(3);
        }
    }
    for i in 1..m {
        let prev = best[i - 1].clone();
        for j in i..n {
            if lower[j] != q[i] {
                continue;
            }
            let mut top = NONE;
            let mut arg = usize::MAX;
            for (k, &before) in prev.iter().enumerate().take(j).skip(i - 1) {
                if before == NONE {
                    continue;
                }
                let link = if k + 1 == j { 6 } else { -((j - k - 1) as i32).min(6) };
                let s = before + link;
                if s > top {
                    top = s;
                    arg = k;
                }
            }
            if top != NONE {
                best[i][j] = top + bonus(j);
                from[i][j] = arg;
            }
        }
    }

    let (end, score) = (0..n).map(|j| (j, best[m - 1][j])).max_by_key(|&(j, s)| (s, std::cmp::Reverse(j)))?;
    if score == NONE {
        return None;
    }
    let mut positions = vec![0; m];
    let mut j = end;
    for i in (0..m).rev() {
        positions[i] = j;
        j = from[i][j];
    }
    let mut result = Match { score: score + if is_run(&positions) { RUN_BONUS } else { 0 }, positions };

    // The DP maximises the score *without* the run bonus, so a contiguous
    // occurrence it passed over can still win once the bonus applies.
    if m > 1 {
        for start in 0..=(n - m) {
            if lower[start..start + m] == q[..] {
                let run: Vec<usize> = (start..start + m).collect();
                let s = path_score(&run, &bonus) + RUN_BONUS;
                if s > result.score {
                    result = Match { score: s, positions: run };
                }
            }
        }
    }
    Some(result)
}

const RUN_BONUS: i32 = 10;

fn is_run(positions: &[usize]) -> bool {
    positions.len() > 1 && positions.windows(2).all(|w| w[1] == w[0] + 1)
}

/// The score of one specific alignment, by the same rules as the DP.
fn path_score(positions: &[usize], bonus: &impl Fn(usize) -> i32) -> i32 {
    let mut score = bonus(positions[0]) - (positions[0] as i32).min(3);
    for w in positions.windows(2) {
        let link = if w[1] == w[0] + 1 { 6 } else { -((w[1] - w[0] - 1) as i32).min(6) };
        score += link + bonus(w[1]);
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rank<'a>(query: &str, items: &[&'a str]) -> Vec<&'a str> {
        let mut scored: Vec<(i32, &str)> =
            items.iter().filter_map(|s| fuzzy_match(query, s).map(|m| (m.score, *s))).collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.len().cmp(&b.1.len())));
        scored.into_iter().map(|(_, s)| s).collect()
    }

    #[test]
    fn subsequence_case_insensitive() {
        assert!(fuzzy_match("OPN", "open file").is_some());
        assert!(fuzzy_match("xyz", "open file").is_none());
        assert!(fuzzy_match("toolong", "tool").is_none());
        assert_eq!(fuzzy_match("", "anything").unwrap().score, 0);
    }

    #[test]
    fn prefers_word_starts_over_inner_letters() {
        let m = fuzzy_match("gp", "Git: Push").unwrap();
        assert_eq!(m.positions, vec![0, 5]);
        assert_eq!(rank("gp", &["Stop group", "Git: Push"])[0], "Git: Push");
    }

    #[test]
    fn prefix_and_runs_beat_scattered() {
        assert_eq!(rank("op", &["Toggle panel", "Open folder"])[0], "Open folder");
        assert_eq!(rank("tsb", &["Toggle side bar", "Test suite: build"])[0], "Toggle side bar");
        assert_eq!(rank("dep", &["Run: Deploy", "Delete paper"])[0], "Run: Deploy");
    }

    #[test]
    fn camel_case_boundaries_count_as_word_starts() {
        let m = fuzzy_match("ws", "WordWrapSettings").unwrap();
        assert_eq!(m.positions, vec![0, 8]);
    }

    #[test]
    fn spaces_in_the_query_are_ignored() {
        assert_eq!(fuzzy_match("git push", "Git: Push").unwrap().positions.len(), 7);
    }
}
