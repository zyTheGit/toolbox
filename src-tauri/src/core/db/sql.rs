//! SQL 文本处理：只允许单条语句，并为分页包装可包装的查询。

/// 返回去掉首尾空白与末尾分号后的单条语句。
/// 多语句、未闭合引号/注释都会报错，避免只读连接上出现意外执行。
pub fn single_statement(sql: &str) -> Result<String, String> {
    let chars: Vec<char> = sql.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut quote: Option<char> = None;
    let mut dollar: Option<String> = None;
    let mut end: Option<usize> = None;

    while i < len {
        let c = chars[i];
        if let Some(tag) = dollar.clone() {
            if c == '$' && starts_with(&chars, i, &tag) {
                i += tag.chars().count();
                dollar = None;
            } else {
                i += 1;
            }
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                if i + 1 < len && chars[i + 1] == q {
                    i += 2;
                    continue;
                }
                quote = None;
            }
            i += 1;
            continue;
        }
        match c {
            '\'' | '"' | '`' => {
                quote = Some(c);
                i += 1;
            }
            '-' if i + 1 < len && chars[i + 1] == '-' => i = skip_line_comment(&chars, i),
            '/' if i + 1 < len && chars[i + 1] == '*' => i = skip_block_comment(&chars, i)?,
            '$' => match dollar_tag(&chars, i) {
                Some((tag, next)) => {
                    dollar = Some(tag);
                    i = next;
                }
                None => i += 1,
            },
            ';' => {
                if end.is_some() {
                    return Err("[PARAM] 一次只能执行一条 SQL 语句".into());
                }
                end = Some(i);
                i += 1;
            }
            _ => i += 1,
        }
    }

    if quote.is_some() {
        return Err("[PARAM] SQL 中存在未闭合的引号".into());
    }
    if dollar.is_some() {
        return Err("[PARAM] SQL 中存在未闭合的 $ 引用".into());
    }

    let stop = end.unwrap_or(len);
    if end.is_some() && !only_trivia(&chars[end.unwrap() + 1..])? {
        return Err("[PARAM] 一次只能执行一条 SQL 语句".into());
    }
    let text: String = chars[..stop].iter().collect();
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("[PARAM] SQL 不能为空".into());
    }
    Ok(text)
}

/// 语句首个关键字（小写）；跳过空白与注释。
pub fn first_keyword(sql: &str) -> String {
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    loop {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i + 1 < chars.len() && chars[i] == '-' && chars[i + 1] == '-' {
            i = skip_line_comment(&chars, i);
        } else if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            i = skip_block_comment(&chars, i).unwrap_or(i + 2);
        } else {
            break;
        }
    }
    chars[i..]
        .iter()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_lowercase()
}

/// 只有 SELECT / WITH / VALUES 的返回值可以安全地包成子查询分页。
pub fn wrappable(keyword: &str) -> bool {
    matches!(keyword, "select" | "with" | "values")
}

pub fn wrap_page(sql: &str, limit: usize, offset: usize) -> String {
    let fetch = limit.saturating_add(1);
    format!("SELECT * FROM (\n{sql}\n) AS toolbox_page LIMIT {fetch} OFFSET {offset}")
}

pub fn describe(keyword: &str) -> String {
    if keyword.is_empty() {
        "未知语句".into()
    } else {
        keyword.to_ascii_uppercase()
    }
}

fn starts_with(chars: &[char], at: usize, needle: &str) -> bool {
    let mut index = at;
    for ch in needle.chars() {
        if chars.get(index) != Some(&ch) {
            return false;
        }
        index += 1;
    }
    true
}

fn skip_line_comment(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && chars[i] != '\n' {
        i += 1;
    }
    i
}

fn skip_block_comment(chars: &[char], start: usize) -> Result<usize, String> {
    let mut i = start + 2;
    while i + 1 < chars.len() {
        if chars[i] == '*' && chars[i + 1] == '/' {
            return Ok(i + 2);
        }
        i += 1;
    }
    Err("[PARAM] SQL 中存在未闭合的块注释".into())
}

fn dollar_tag(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut i = start + 1;
    while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
        i += 1;
    }
    if chars.get(i) == Some(&'$') {
        let tag: String = chars[start..=i].iter().collect();
        Some((tag, i + 1))
    } else {
        None
    }
}

/// 分号之后是否只剩空白或注释。
fn only_trivia(chars: &[char]) -> Result<bool, String> {
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '-' && chars.get(i + 1) == Some(&'-') {
            i = skip_line_comment(chars, i);
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i = skip_block_comment(chars, i)?;
        } else {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_single_select() {
        assert_eq!(single_statement(" SELECT 1; ").unwrap(), "SELECT 1");
        assert_eq!(single_statement("select ';' as x").unwrap(), "select ';' as x");
    }

    #[test]
    fn rejects_multiple_statements() {
        assert!(single_statement("select 1; select 2").is_err());
        assert!(single_statement("select 1;;").is_err());
    }

    #[test]
    fn handles_comments_quotes_and_dollar_tags() {
        assert_eq!(
            single_statement("select '--not comment' /* ; */").unwrap(),
            "select '--not comment' /* ; */"
        );
        assert_eq!(
            single_statement("select $$a;b$$").unwrap(),
            "select $$a;b$$"
        );
        assert_eq!(single_statement("select 1; -- done").unwrap(), "select 1");
    }

    #[test]
    fn detects_first_keyword() {
        assert_eq!(first_keyword(" /* c */ -- x\n WITH t AS (SELECT 1) SELECT * FROM t"), "with");
        assert_eq!(first_keyword("PRAGMA table_info(t)"), "pragma");
    }

    #[test]
    fn wraps_paginable_queries() {
        assert!(wrappable(&first_keyword("select * from t")));
        assert!(!wrappable(&first_keyword("explain select 1")));
        let wrapped = wrap_page("select 1", 20, 40);
        assert!(wrapped.contains("LIMIT 21 OFFSET 40"));
    }
}
