# Recording a Why review

This is a manual, auditable review procedure built on existing knowledge records and revision-pinned relations. It records a reviewer's claim; Riltra does not authenticate the author, check that the review was performed, or verify that a conclusion is true. Record verification remains `Unverified`. This is a partial workflow, not a built-in review state or a certification mechanism.

## Procedure

1. Read the exact target record revision and its evidence using `knowledge show`; inspect the actual source, decision and supporting material. Record the target ID and revision, source snapshot or digest, reviewer, review date, scope, evidence examined, conclusion and reasoning. Use `inconclusive` when evidence is insufficient.
2. Create a separate knowledge record containing those details. Its `author` is the asserted reviewer and its `evidence` contains the repository paths actually examined. Evidence freshness describes the stored source reference, not the correctness of the conclusion. Do not record a review on behalf of another person.
3. Create an explicit `relates_to` relation from the review record revision to the exact reviewed record revision. Record the review outcome in the relation rationale. The relation preserves the claimed review and author; it does not automatically approve or verify its target.
4. Read the target with `knowledge related --revision <revision> --direction inbound`, then read each review record at its pinned revision. Check retractions, evidence freshness, reviewer identity through an external trusted channel, and whether the review's scope covers the question. A later record revision or changed source needs a new review; never carry a conclusion over silently.

For example, after creating a review record `review-1` revision 1, the following input links it to `decision-1` revision 2:

```json
{"relation_id":"review-link-1","src_id":"review-1","src_revision":1,"dst_id":"decision-1","dst_revision":2,"kind":"relates_to","author":"reviewer","rationale":"Inconclusive: evidence reviewed is insufficient to decide."}
```

Pass that JSON on stdin to `riltra knowledge link --repo <repo> --input -`. `riltra knowledge related --repo <repo> --id decision-1 --revision 2 --direction inbound` retrieves the link; `riltra knowledge show --repo <repo> --id review-1 --revision 1` retrieves the review. The corresponding MCP read tools are `riltra_knowledge_related` and `riltra_knowledge_show`.

## Limitations and evidence

The store does not enforce review fields, validate reviewer identity, require independent reviewers, prevent a review record from making unsupported claims, or derive a reviewed state from links. `knowledge verify` checks store structure and is not content verification. Audit the original revision, relation history, retractions and evidence rather than treating a link or `confirm_state` as a truth verdict.

A local synthetic fixture created an `inconclusive` review record and a `relates_to` relation to `e2-codex-confined-read-pr14` revision 1 in a copy of the provider evaluation store. CLI and MCP `related` responses were identical; the review record remained `Unverified` and `candidate`. The fixture is a test of the recording and retrieval path, not a review of the real decision. Its inputs and outputs are retained in the isolated follow-up run directory; they must be preserved and sanitized before sharing.
