//! Lightweight in-memory search for MCP tools
//!
//! This module provides fast, fuzzy search capabilities for tools, resources, and prompts
//! using a combination of string similarity and keyword matching.

use std::collections::HashMap;

/// Search result with relevance scoring
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub index: usize,
    pub score: f32,
    pub match_reason: String,
}

/// Searchable item that can be indexed
#[derive(Debug, Clone)]
pub struct SearchableItem {
    pub name: String,
    pub description: String,
    pub category: SearchCategory,
}

/// Category of searchable items
#[derive(Debug, Clone, PartialEq)]
pub enum SearchCategory {
    Tool,
    Resource,
    Prompt,
}

/// Lightweight search engine for MCP capabilities
pub struct SearchEngine {
    items: Vec<SearchableItem>,
    name_index: HashMap<String, Vec<usize>>,
    keyword_index: HashMap<String, Vec<usize>>,
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            name_index: HashMap::new(),
            keyword_index: HashMap::new(),
        }
    }

    /// Index tools for searching
    pub fn index_tools(&mut self, tools: &[crate::tui::ToolInfo]) {
        for tool in tools.iter() {
            let keywords =
                self.extract_keywords(&tool.name, &tool.description.clone().unwrap_or_default());
            let item = SearchableItem {
                name: tool.name.clone(),
                description: tool.description.clone().unwrap_or_default(),
                category: SearchCategory::Tool,
            };

            let item_index = self.items.len();
            self.items.push(item);

            // Index by name tokens
            for token in self.tokenize(&tool.name) {
                self.name_index
                    .entry(token.to_lowercase())
                    .or_default()
                    .push(item_index);
            }

            // Index by keywords
            for keyword in keywords {
                self.keyword_index
                    .entry(keyword.to_lowercase())
                    .or_default()
                    .push(item_index);
            }
        }
    }

    /// Index resources for searching
    pub fn index_resources(&mut self, resources: &[crate::tui::ResourceInfo]) {
        let _tool_count = self
            .items
            .iter()
            .filter(|i| i.category == SearchCategory::Tool)
            .count();

        for resource in resources.iter() {
            let keywords = self.extract_keywords(
                &resource.uri,
                &resource.description.clone().unwrap_or_default(),
            );
            let item = SearchableItem {
                name: resource.uri.clone(),
                description: resource.description.clone().unwrap_or_default(),
                category: SearchCategory::Resource,
            };

            let item_index = self.items.len();
            self.items.push(item);

            // Index by name tokens
            for token in self.tokenize(&resource.uri) {
                self.name_index
                    .entry(token.to_lowercase())
                    .or_default()
                    .push(item_index);
            }

            // Index by keywords
            for keyword in keywords {
                self.keyword_index
                    .entry(keyword.to_lowercase())
                    .or_default()
                    .push(item_index);
            }
        }
    }

    /// Index prompts for searching
    pub fn index_prompts(&mut self, prompts: &[crate::tui::PromptInfo]) {
        let _existing_count = self.items.len();

        for prompt in prompts.iter() {
            let keywords = self.extract_keywords(
                &prompt.name,
                &prompt.description.clone().unwrap_or_default(),
            );
            let item = SearchableItem {
                name: prompt.name.clone(),
                description: prompt.description.clone().unwrap_or_default(),
                category: SearchCategory::Prompt,
            };

            let item_index = self.items.len();
            self.items.push(item);

            // Index by name tokens
            for token in self.tokenize(&prompt.name) {
                self.name_index
                    .entry(token.to_lowercase())
                    .or_default()
                    .push(item_index);
            }

            // Index by keywords
            for keyword in keywords {
                self.keyword_index
                    .entry(keyword.to_lowercase())
                    .or_default()
                    .push(item_index);
            }
        }
    }

    /// Search for items matching the query
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let query_tokens = self.tokenize(&query_lower);

        let mut scores: HashMap<usize, (f32, String)> = HashMap::new();

        // Exact name matches (highest priority)
        for (index, item) in self.items.iter().enumerate() {
            if item.name.to_lowercase().contains(&query_lower) {
                let score = if item.name.to_lowercase() == query_lower {
                    100.0 // Perfect match
                } else if item.name.to_lowercase().starts_with(&query_lower) {
                    90.0 // Prefix match
                } else {
                    80.0 // Contains match
                };
                scores.insert(index, (score, "Name match".to_string()));
            }
        }

        // Description matches
        for (index, item) in self.items.iter().enumerate() {
            if !scores.contains_key(&index)
                && item.description.to_lowercase().contains(&query_lower)
            {
                scores.insert(index, (70.0, "Description match".to_string()));
            }
        }

        // Token-based fuzzy matching
        for token in &query_tokens {
            // Direct token matches in name index
            if let Some(indices) = self.name_index.get(token) {
                for &index in indices {
                    scores
                        .entry(index)
                        .or_insert_with(|| (60.0, "Token match".to_string()));
                }
            }

            // Keyword matches
            if let Some(indices) = self.keyword_index.get(token) {
                for &index in indices {
                    scores
                        .entry(index)
                        .or_insert_with(|| (50.0, "Keyword match".to_string()));
                }
            }

            // Fuzzy token matching
            for (key, indices) in &self.name_index {
                if self.fuzzy_match(token, key) > 0.7 {
                    for &index in indices {
                        scores.entry(index).or_insert_with(|| {
                            let similarity = self.fuzzy_match(token, key);
                            (similarity * 40.0, "Fuzzy match".to_string())
                        });
                    }
                }
            }
        }

        // Convert to results and sort by score
        let mut results: Vec<SearchResult> = scores
            .into_iter()
            .map(|(index, (score, reason))| SearchResult {
                index,
                score,
                match_reason: reason,
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        results
    }

    /// Get item by index
    pub fn get_item(&self, index: usize) -> Option<&SearchableItem> {
        self.items.get(index)
    }

    /// Get total number of indexed items
    pub fn total_items(&self) -> usize {
        self.items.len()
    }

    /// Extract keywords from name and description
    fn extract_keywords(&self, name: &str, description: &str) -> Vec<String> {
        let mut keywords = Vec::new();

        // Extract from name (split by common separators)
        let name_tokens = self.tokenize(name);
        keywords.extend(name_tokens.into_iter().map(|s| s.to_lowercase()));

        // Extract important words from description
        let desc_words: Vec<String> = description
            .split_whitespace()
            .filter(|word| word.len() > 3) // Only meaningful words
            .filter(|word| !self.is_stopword(word))
            .take(10) // Limit to first 10 meaningful words
            .map(|word| {
                word.to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|word| !word.is_empty())
            .collect();

        keywords.extend(desc_words);
        keywords.sort();
        keywords.dedup();
        keywords
    }

    /// Tokenize text into searchable tokens
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|token| !token.is_empty() && token.len() > 1)
            .map(|token| token.to_string())
            .collect()
    }

    /// Simple fuzzy string matching using Jaro similarity
    fn fuzzy_match(&self, a: &str, b: &str) -> f32 {
        if a == b {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        let match_distance = (a_chars.len().max(b_chars.len()) / 2).saturating_sub(1);
        let mut a_matches = vec![false; a_chars.len()];
        let mut b_matches = vec![false; b_chars.len()];

        let mut matches = 0;

        // Find matches
        for i in 0..a_chars.len() {
            let start = i.saturating_sub(match_distance);
            let end = (i + match_distance + 1).min(b_chars.len());

            for j in start..end {
                if b_matches[j] || a_chars[i] != b_chars[j] {
                    continue;
                }
                a_matches[i] = true;
                b_matches[j] = true;
                matches += 1;
                break;
            }
        }

        if matches == 0 {
            return 0.0;
        }

        // Count transpositions
        let mut transpositions = 0;
        let mut k = 0;
        for i in 0..a_chars.len() {
            if !a_matches[i] {
                continue;
            }
            while !b_matches[k] {
                k += 1;
            }
            if a_chars[i] != b_chars[k] {
                transpositions += 1;
            }
            k += 1;
        }

        (matches as f32 / a_chars.len() as f32
            + matches as f32 / b_chars.len() as f32
            + (matches - transpositions / 2) as f32 / matches as f32)
            / 3.0
    }

    /// Check if word is a common stopword
    fn is_stopword(&self, word: &str) -> bool {
        matches!(
            word.to_lowercase().as_str(),
            "the"
                | "a"
                | "an"
                | "and"
                | "or"
                | "but"
                | "in"
                | "on"
                | "at"
                | "to"
                | "for"
                | "of"
                | "with"
                | "by"
                | "is"
                | "are"
                | "was"
                | "were"
                | "be"
                | "been"
                | "have"
                | "has"
                | "had"
                | "do"
                | "does"
                | "did"
                | "will"
                | "would"
                | "could"
                | "should"
                | "this"
                | "that"
                | "these"
                | "those"
                | "it"
                | "its"
                | "they"
                | "them"
                | "their"
        )
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        let engine = SearchEngine::new();

        // Perfect match
        assert!((engine.fuzzy_match("hello", "hello") - 1.0).abs() < 0.01);

        // Similar strings
        assert!(engine.fuzzy_match("hello", "helo") > 0.8);
        assert!(engine.fuzzy_match("github", "gitub") > 0.8);

        // Different strings
        assert!(engine.fuzzy_match("hello", "world") < 0.5);
    }

    #[test]
    fn test_tokenize() {
        let engine = SearchEngine::new();
        let tokens = engine.tokenize("github.repos/create-commit-status");

        assert!(tokens.contains(&"github".to_string()));
        assert!(tokens.contains(&"repos".to_string()));
        assert!(tokens.contains(&"create".to_string()));
        assert!(tokens.contains(&"commit".to_string()));
        assert!(tokens.contains(&"status".to_string()));
    }
}
