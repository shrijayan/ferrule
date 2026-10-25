//! Class names of inline objects. Reduced from the isaacus oracle, where
//! `UniversalClassificationResponse.classifications[].chunks` is `oneOf: [array, null]` and its
//! item class is `ClassificationChunk`.

use ferrule_python_plugin::emit::models::{ModelFile, emit_model_file};
use ferrule_python_plugin::model::{SdkField, SdkObject, SdkPrimitive, SdkRef, SdkType};

fn field(name: &str, required: bool, ty: SdkType) -> SdkField {
    SdkField {
        wire_name: name.into(),
        py_name: name.into(),
        required,
        description: String::new(),
        ty,
    }
}

#[test]
fn names_the_items_of_a_nullable_array_in_the_singular() {
    let chunk = SdkType::Object(SdkObject {
        fields: vec![field("index", true, SdkType::Primitive(SdkPrimitive::Integer))],
        ..SdkObject::default()
    });
    let chunks = SdkType::Union {
        variants: vec![SdkType::Array(Box::new(chunk)), SdkType::Primitive(SdkPrimitive::Null)],
        discriminator: None,
    };
    let response = SdkObject {
        fields: vec![field("chunks", false, chunks)],
        ..SdkObject::default()
    };
    let r = SdkRef {
        name: "ClassificationResponse".into(),
        module: "classification_response".into(),
    };
    let text = emit_model_file(&ModelFile {
        r#ref: &r,
        ty: &response,
        prefix_children: false,
        cycles: None,
    });
    assert!(text.contains("class Chunk(BaseModel):"), "{text}");
    assert!(text.contains("chunks: Optional[List[Chunk]] = None"), "{text}");
}

/// Class names made from field names spell the default initialisms of the stainless.yml schema
/// upper-case (knock-python `types/recipients/preference_set_channel_types.py` `SMS` and
/// `HTTP`), and `json` like any word (groq-python and perplexity-py `completion_create_params.py`
/// `ResponseFormatResponseFormatJsonSchemaJsonSchema` from `json_schema`).
#[test]
fn spells_the_default_initialisms_in_field_class_names() {
    use ferrule_python_plugin::naming::class_segment;
    assert_eq!(class_segment(&["sms"]), "SMS");
    assert_eq!(class_segment(&["http"]), "HTTP");
    assert_eq!(class_segment(&["json", "schema"]), "JsonSchema");
    assert_eq!(class_segment(&["situation", "ids"]), "SituationIDs");
}
