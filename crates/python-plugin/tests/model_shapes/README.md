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
- `titled_array_variant`: `RequestTemplate.headers`, a `oneOf` of a string and an array of
  objects titled `RequestTemplateHeadersArray` (knock-mgmt-python `types/request_template.py`).
  The converter points the member at the equal `RequestTemplateHeadersArray` schema
  (`title-hoisted`); the items are `HeadersRequestTemplateHeadersArray`, the field's class name
  followed by the array's title, without a docstring, and the field is
  `Union[str, List[HeadersRequestTemplateHeadersArray], None]`.
- `string_or_object`: `workflows.run`, whose body lists `recipients` and an optional `actor`,
  each a `RecipientReference`: a `oneOf` of a string and an object titled
  `ObjectRecipientReference` (knock-mgmt-python `types/workflow_run_params.py`,
  `resources/workflows/workflows.py`). Each field gets an alias named after it (singular in a
  list), `Recipient: TypeAlias = Union[str, RecipientObjectRecipientReference]`, declared after
  its object; `actor` is `Optional[Actor]` (the alias leaves `None` out) and `recipients` is
  `SequenceNotStr[Recipient]`, since a member is a string. The fixture gives `actor` the
  description of `RecipientReference`, which Stainless prints there.
- `reserved_field_name`: `MessageTypeJsonField` with a `schema` property (knock-mgmt-python
  `types/shared/message_type_json_field.py` `Settings.schema_`, `types/shared_params/...`
  `schema`; imagekit-python `types/custom_metadata_field.py`). In a model it would shadow
  `BaseModel.schema`, so it is `schema_ = FieldInfo(alias="schema", ...)`; the params
  `TypedDict` keeps `schema`. Typed `string` here (the oracle's is a bare `type: object`).
- `protected_field_names`: a `Tenant` with `__typename` (knock-python `types/tenant.py`
  `api_typename`) and `model_id` (ogx-client-python `types/vector_stores/vector_store_file.py`
  `api_model_id`; opencode-sdk-python `types/mode.py`). In a model, a name Pydantic keeps for
  itself, private (leading `_`) or protected (`model_`), is prefixed `api_` with the wire name as
  the alias; the params `TypedDict` keeps `model_id` (opencode-sdk-python
  `types/session_init_params.py`). This one is from knock, ogx and opencode, not knock-mgmt.
- `plural_items`: `WorkflowBranchStep.branches` and `SourceProviderResponse.example_payloads`, a
  map of lists of objects, in one model (knock-mgmt-python `types/workflow_branch_step.py`
  `Branch`, `types/source_provider_response.py` `ExamplePayload`). Item classes are named in the
  singular: `-ches`, `-shes`, `-xes`, `-sses` and `-zzes` lose `es`, and a map of lists names its
  items as a list does. The fixture leaves out the items' own description, which Forge does not
  expose (gap G20; the oracle's `Branch` has it as a docstring).
- `extra_properties`: `SourceEnvironmentSettings.settings`, an object with properties and
  `additionalProperties: true` (knock-mgmt-python `types/source_environment_settings.py`). After
  its fields the class declares, under `if TYPE_CHECKING:`, `__pydantic_extra__: Dict[str,
  object] = FieldInfo(init=False)` and a `__getattr__` stub returning the extra values' type,
  and `__pydantic_extra__: Dict[str, object]` otherwise.
