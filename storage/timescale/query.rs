use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Query builder for common time-series query patterns
pub struct QueryBuilder {
    table: String,
    conditions: Vec<String>,
    params: Vec<String>,
    order_by: Option<String>,
    limit: Option<usize>,
}

impl QueryBuilder {
    /// Create a new query builder for a table
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            conditions: Vec::new(),
            params: Vec::new(),
            order_by: None,
            limit: None,
        }
    }

    /// Add time range filter
    pub fn time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let start_param = format!("${}", self.params.len() + 1);
        let end_param = format!("${}", self.params.len() + 2);

        self.conditions
            .push(format!("timestamp >= {} AND timestamp <= {}", start_param, end_param));
        self.params.push(start.to_rfc3339());
        self.params.push(end.to_rfc3339());
        self
    }

    /// Add equals condition
    pub fn eq(mut self, column: &str, value: &str) -> Self {
        let param = format!("${}", self.params.len() + 1);
        self.conditions.push(format!("{} = {}", column, param));
        self.params.push(value.to_string());
        self
    }

    /// Add LIKE condition
    pub fn like(mut self, column: &str, pattern: &str) -> Self {
        let param = format!("${}", self.params.len() + 1);
        self.conditions.push(format!("{} LIKE {}", column, param));
        self.params.push(pattern.to_string());
        self
    }

    /// Add IN condition
    pub fn in_list(mut self, column: &str, values: &[String]) -> Self {
        if values.is_empty() {
            return self;
        }

        let placeholders: Vec<String> = values
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.params.len() + i + 1))
            .collect();

        self.conditions.push(format!("{} IN ({})", column, placeholders.join(", ")));
        self.params.extend_from_slice(values);
        self
    }

    /// Add JSONB label filter
    pub fn labels(mut self, labels: &HashMap<String, String>) -> Self {
        if labels.is_empty() {
            return self;
        }

        for (key, value) in labels {
            let param = format!("${}", self.params.len() + 1);
            self.conditions.push(format!("labels->>'{}' = {}", key, param));
            self.params.push(value.clone());
        }
        self
    }

    /// Add JSONB containment filter (@>)
    pub fn labels_contains(mut self, labels_json: &str) -> Self {
        let param = format!("${}", self.params.len() + 1);
        self.conditions.push(format!("labels @> {}::jsonb", param));
        self.params.push(labels_json.to_string());
        self
    }

    /// Set order by clause
    pub fn order_by(mut self, column: &str, desc: bool) -> Self {
        self.order_by = Some(format!("{} {}", column, if desc { "DESC" } else { "ASC" }));
        self
    }

    /// Set limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Build SELECT query
    pub fn build_select(&self, columns: &[&str]) -> (String, Vec<String>) {
        let cols = if columns.is_empty() {
            "*".to_string()
        } else {
            columns.join(", ")
        };

        let mut query = format!("SELECT {} FROM {}", cols, self.table);

        if !self.conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", self.conditions.join(" AND ")));
        }

        if let Some(ref order) = self.order_by {
            query.push_str(&format!(" ORDER BY {}", order));
        }

        if let Some(limit) = self.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        (query, self.params.clone())
    }

    /// Build COUNT query
    pub fn build_count(&self) -> (String, Vec<String>) {
        let mut query = format!("SELECT COUNT(*) FROM {}", self.table);

        if !self.conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", self.conditions.join(" AND ")));
        }

        (query, self.params.clone())
    }

    /// Build DELETE query
    pub fn build_delete(&self) -> (String, Vec<String>) {
        let mut query = format!("DELETE FROM {}", self.table);

        if !self.conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", self.conditions.join(" AND ")));
        }

        (query, self.params.clone())
    }
}

/// Builder for aggregation queries
pub struct AggregationQueryBuilder {
    table: String,
    time_column: String,
    bucket_interval: String,
    group_by: Vec<String>,
    conditions: Vec<String>,
    params: Vec<String>,
}

impl AggregationQueryBuilder {
    /// Create new aggregation query builder
    pub fn new(table: &str, bucket_interval: &str) -> Self {
        Self {
            table: table.to_string(),
            time_column: "timestamp".to_string(),
            bucket_interval: bucket_interval.to_string(),
            group_by: Vec::new(),
            conditions: Vec::new(),
            params: Vec::new(),
        }
    }

    /// Set time column (default: timestamp)
    pub fn time_column(mut self, column: &str) -> Self {
        self.time_column = column.to_string();
        self
    }

    /// Add time range filter
    pub fn time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let start_param = format!("${}", self.params.len() + 1);
        let end_param = format!("${}", self.params.len() + 2);

        self.conditions.push(format!(
            "{} >= {} AND {} <= {}",
            self.time_column, start_param, self.time_column, end_param
        ));
        self.params.push(start.to_rfc3339());
        self.params.push(end.to_rfc3339());
        self
    }

    /// Add group by column
    pub fn group_by(mut self, column: &str) -> Self {
        self.group_by.push(column.to_string());
        self
    }

    /// Add WHERE condition
    pub fn where_eq(mut self, column: &str, value: &str) -> Self {
        let param = format!("${}", self.params.len() + 1);
        self.conditions.push(format!("{} = {}", column, param));
        self.params.push(value.to_string());
        self
    }

    /// Build aggregation query with statistics
    pub fn build_stats(&self, value_column: &str) -> (String, Vec<String>) {
        let mut select_parts = vec![
            format!("time_bucket('{}', {}) AS bucket", self.bucket_interval, self.time_column),
            format!("AVG({}) AS avg_value", value_column),
            format!("MIN({}) AS min_value", value_column),
            format!("MAX({}) AS max_value", value_column),
            format!("COUNT(*) AS count", ),
        ];

        // Add group by columns to select
        for col in &self.group_by {
            select_parts.push(col.clone());
        }

        let mut query = format!("SELECT {} FROM {}", select_parts.join(", "), self.table);

        if !self.conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", self.conditions.join(" AND ")));
        }

        let mut group_parts = vec!["bucket".to_string()];
        group_parts.extend(self.group_by.clone());

        query.push_str(&format!(" GROUP BY {}", group_parts.join(", ")));
        query.push_str(" ORDER BY bucket DESC");

        (query, self.params.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_query_builder_select() {
        let now = Utc::now();
        let start = now - Duration::hours(1);

        let (query, params) = QueryBuilder::new("metrics")
            .time_range(start, now)
            .eq("metric_name", "test_metric")
            .order_by("timestamp", true)
            .limit(100)
            .build_select(&["timestamp", "value"]);

        assert!(query.contains("SELECT timestamp, value FROM metrics"));
        assert!(query.contains("WHERE"));
        assert!(query.contains("ORDER BY timestamp DESC"));
        assert!(query.contains("LIMIT 100"));
        assert_eq!(params.len(), 3); // start, end, metric_name
    }

    #[test]
    fn test_query_builder_labels() {
        let mut labels = HashMap::new();
        labels.insert("venue".to_string(), "polymarket".to_string());

        let (query, params) = QueryBuilder::new("metrics")
            .labels(&labels)
            .build_select(&["*"]);

        assert!(query.contains("labels->>'venue' = $1"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "polymarket");
    }

    #[test]
    fn test_aggregation_builder() {
        let now = Utc::now();
        let start = now - Duration::hours(24);

        let (query, params) = AggregationQueryBuilder::new("metrics", "1 hour")
            .time_range(start, now)
            .group_by("metric_name")
            .build_stats("value");

        assert!(query.contains("time_bucket('1 hour', timestamp)"));
        assert!(query.contains("AVG(value)"));
        assert!(query.contains("GROUP BY bucket, metric_name"));
        assert_eq!(params.len(), 2); // start, end
    }
}
