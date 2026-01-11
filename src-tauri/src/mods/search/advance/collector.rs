use tantivy::{
    collector::Collector, columnar::StrColumn, postings::SegmentPostings, schema::{Field, IndexRecordOption}, DocAddress, DocId, DocSet, Score, SegmentReader, Term
};
use std::{cmp::Ordering, collections::HashMap};

use crate::types::Id;

pub struct MatchedFieldsCollector {
    limit: usize,
    fields: Vec<Field>,
    terms: Vec<String>,
}

impl MatchedFieldsCollector {
    pub fn new(limit: usize, fields: Vec<Field>, terms: Vec<String>) -> Self {
        Self {
            limit,
            fields,
            terms,
        }
    }
}

impl Collector for MatchedFieldsCollector {
    type Fruit = Vec<(Score, DocAddress, Id, Vec<Field>)>;
    type Child = MatchedFieldsSegmentCollector;

    fn for_segment(
        &self,
        segment_local_id: u32,
        segment_reader: &SegmentReader,
    ) -> tantivy::Result<Self::Child> {
        let mut field_term_postings = Vec::new();

        // Initialize postings readers for all field-term combinations
        for field in &self.fields {
            if let Ok(inverted_index) = segment_reader.inverted_index(*field) {
                for term in &self.terms {
                    if let Ok(Some(postings)) = inverted_index.read_postings(
                        &Term::from_field_text(*field, term),
                        IndexRecordOption::Basic,
                    ) {
                        field_term_postings.push((*field, term.clone(), postings));
                    }
                }
            }
        }
        Ok(MatchedFieldsSegmentCollector {
            limit: self.limit,
            fields: self.fields.clone(),
            results: Vec::with_capacity(self.limit),
            segment_ord: segment_local_id,
            field_term_postings,
            id_column: segment_reader.fast_fields().str("id")?.unwrap(),
        })
    }

    fn merge_fruits(&self, segment_fruits: Vec<Self::Fruit>) -> tantivy::Result<Self::Fruit> {
        let mut results = Vec::new();
        for fruit in segment_fruits {
            results.extend(fruit);
        }
        results.sort_by(
            |(score_a, _, id_a, _), (score_b, _, id_b, _)| 
                score_b.partial_cmp(score_a).unwrap_or(Ordering::Equal).then(id_a.cmp(id_b))
            );
        results.truncate(self.limit);
        Ok(results)
    }

    fn requires_scoring(&self) -> bool {
        true
    }
}

pub struct MatchedFieldsSegmentCollector {
    limit: usize,
    fields: Vec<Field>,
    results: Vec<(Score, DocAddress, Id, Vec<Field>)>,
    segment_ord: u32,
    field_term_postings: Vec<(Field, String, SegmentPostings)>,
    id_column: StrColumn 
}

impl tantivy::collector::SegmentCollector for MatchedFieldsSegmentCollector {
    type Fruit = Vec<(Score, DocAddress, Id, Vec<Field>)>;

    fn collect(&mut self, doc: DocId, score: Score) -> () {
        let doc_address = DocAddress::new(self.segment_ord, doc);
        let mut matched_fields = Vec::new();

        // Group postings by field
        let mut field_matches: HashMap<Field, bool> = HashMap::new();

        for (field, _term, postings) in &mut self.field_term_postings {
            // Advance the postings to the current document
            /*             while let Some(posting_doc) = postings.doc() {
                if posting_doc < doc {
                    postings.advance();
                } else if posting_doc == doc {
                    // Found a match for this field
                    field_matches.insert(*field, true);
                    break;
                } else {
                    // Document not found in this posting list
                    break;
                }
            } */
            //dbg!(doc);
            //dbg!(postings.doc());
            if doc < postings.doc() {
                continue;
            }
            let re = postings.seek(doc);
            if re == doc {
                field_matches.insert(*field, true);
            }
        }

        // Collect all matched fields
        for &field in &self.fields {
            if field_matches.get(&field).copied().unwrap_or(false) {
                matched_fields.push(field);
            }
        }

        if !matched_fields.is_empty() {
            let mut id = String::new();
            self.id_column.ord_to_str(doc as u64, &mut id).unwrap(); // TODO ???
            let id = Id::from(id);
            self.results.push((score, doc_address, id, matched_fields));
        }
    }

    fn harvest(self) -> Self::Fruit {
        let mut t = self.results;
        t.sort_by(
            |(score_a, _, id_a, _), (score_b, _, id_b, _)| 
                score_b.partial_cmp(score_a).unwrap_or(Ordering::Equal).then(id_a.cmp(id_b))
            );
        t.truncate(self.limit);
        t
    }
}