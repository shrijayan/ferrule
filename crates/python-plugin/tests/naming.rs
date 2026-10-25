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
