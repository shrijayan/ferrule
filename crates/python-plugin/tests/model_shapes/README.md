# model_shapes

Model and params shapes, one directory per shape. Each holds an `openapi.yml` and `stainless.yml`
reduced from knock-mgmt (knocklabs/docs `data/specs/mapi`, phase 4 triage), and `annotated.json`,
which is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`. The expected Python is
in `../golden/model-shapes/<shape>/` and pinned by `../model_shapes.rs`; it was written from the
oracle file named below (knock-mgmt-python 0.1.0-alpha.2), not generated.

- `variant_names`: `EmailTemplate.visual_blocks`, an `anyOf: [$ref EmailVisualBlocks, {nullable:
  true}]` whose items are a union of `$ref`s (knock-mgmt-python `types/email_template.py`). The
  variants are named after their schemas, with the default initialisms of the stainless.yml
  schema upper-case: `EmailHtmlBlock` is `VisualBlockEmailHTMLBlock`, and its nested object
  `VisualBlockEmailHTMLBlockLayoutAttrs`. Other words are capitalised, the rest lower-case, so an
  upper-case run that is not an initialism is spelled as a word (ogx-client-python
  `ToolChoiceOpenAIResponseInputToolChoiceMcpTool` from the schema
  `OpenAIResponseInputToolChoiceMCPTool`).
