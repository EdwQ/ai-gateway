use crate::redis::RedisPool;

/// Simple rate limiting using Redis sliding window (1-second window).
/// Returns Ok if within limit, Err if exceeded.
pub async fn check_rate_limit(
    redis: &RedisPool,
    key_prefix: &str,
    max_qps: u32,
) -> Result<(), String> {
    let key = format!("ratelimit:{}", key_prefix);
    let mut conn = redis.conn.clone();

    // INCR key
    let current: i64 = redis::cmd("INCR")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| format!("Redis error: {}", e))?;

    if current == 1 {
        // First request in window, set expiry
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(1)
            .query_async(&mut conn)
            .await
            .map_err(|e| format!("Redis error: {}", e))?;
    }

    if current > max_qps as i64 {
        return Err("Rate limit exceeded. Please slow down.".to_string());
    }

    Ok(())
}
