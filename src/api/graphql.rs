//! GraphQL query helpers for Cloudflare Analytics API

/// Build a GraphQL query for HTTP requests analytics
pub fn http_requests_query(
    zone_id: &str,
    since: &str,
    limit: u32,
    dimensions: &[&str],
    order_by: &str,
    filter: Option<&str>,
) -> String {
    let dims = dimensions.join(" ");
    let filter_clause = filter
        .map(|f| format!(", {}", f))
        .unwrap_or_default();
    
    format!(
        r#"{{
  viewer {{
    zones(filter: {{zoneTag: "{zone_id}"}}) {{
      httpRequestsAdaptiveGroups(
        limit: {limit},
        filter: {{datetime_geq: "{since}"{filter_clause}}},
        orderBy: [{order_by}]
      ) {{
        count
        dimensions {{
          {dims}
        }}
      }}
    }}
  }}
}}"#
    )
}

/// Build a GraphQL query for firewall events
pub fn firewall_events_query(
    zone_id: &str,
    since: &str,
    limit: u32,
) -> String {
    format!(
        r#"{{
  viewer {{
    zones(filter: {{zoneTag: "{zone_id}"}}) {{
      firewallEventsAdaptiveGroups(
        limit: {limit},
        filter: {{datetime_geq: "{since}"}},
        orderBy: [count_DESC]
      ) {{
        count
        dimensions {{
          action
          clientIP
          clientCountryName
          clientRequestPath
          ruleId
          source
        }}
      }}
    }}
  }}
}}"#
    )
}

/// Build query for top URLs
pub fn top_urls_query(zone_id: &str, since: &str, limit: u32) -> String {
    http_requests_query(
        zone_id,
        since,
        limit,
        &["clientRequestPath"],
        "count_DESC",
        None,
    )
}

/// Build query for top IPs
pub fn top_ips_query(zone_id: &str, since: &str, limit: u32) -> String {
    http_requests_query(
        zone_id,
        since,
        limit,
        &["clientIP", "clientCountryName", "clientASNDescription"],
        "count_DESC",
        None,
    )
}

/// Build query for top countries
pub fn top_countries_query(zone_id: &str, since: &str, limit: u32) -> String {
    http_requests_query(
        zone_id,
        since,
        limit,
        &["clientCountryName"],
        "count_DESC",
        None,
    )
}

/// Build query for error responses (4xx, 5xx)
pub fn errors_query(zone_id: &str, since: &str, limit: u32) -> String {
    http_requests_query(
        zone_id,
        since,
        limit,
        &["edgeResponseStatus", "clientRequestPath", "clientIP"],
        "count_DESC",
        Some("edgeResponseStatus_geq: 400"),
    )
}

/// Build query for cache status
pub fn cache_status_query(zone_id: &str, since: &str, limit: u32) -> String {
    http_requests_query(
        zone_id,
        since,
        limit,
        &["cacheStatus"],
        "count_DESC",
        None,
    )
}

/// Build query for bandwidth by content type
pub fn bandwidth_query(zone_id: &str, since: &str, limit: u32) -> String {
    format!(
        r#"{{
  viewer {{
    zones(filter: {{zoneTag: "{zone_id}"}}) {{
      httpRequestsAdaptiveGroups(
        limit: {limit},
        filter: {{datetime_geq: "{since}"}},
        orderBy: [sum_edgeResponseBytes_DESC]
      ) {{
        sum {{
          edgeResponseBytes
        }}
        dimensions {{
          edgeResponseContentTypeName
        }}
      }}
    }}
  }}
}}"#
    )
}

/// Build query for bots
pub fn bots_query(zone_id: &str, since: &str, limit: u32) -> String {
    http_requests_query(
        zone_id,
        since,
        limit,
        &["clientDeviceType", "botScoreSrcName"],
        "count_DESC",
        Some("botScore_leq: 30"),
    )
}

/// Build query for hourly traffic
pub fn hourly_traffic_query(zone_id: &str, since: &str) -> String {
    format!(
        r#"{{
  viewer {{
    zones(filter: {{zoneTag: "{zone_id}"}}) {{
      httpRequests1hGroups(
        limit: 168,
        filter: {{datetime_geq: "{since}"}},
        orderBy: [datetime_ASC]
      ) {{
        sum {{
          requests
          bytes
          cachedBytes
          threats
        }}
        dimensions {{
          datetime
        }}
      }}
    }}
  }}
}}"#
    )
}

