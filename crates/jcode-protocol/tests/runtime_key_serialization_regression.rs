use jcode_protocol::Request;
use jcode_provider_core::{
    CHATGPT_WEB_MODEL, ModelRoute, ModelRouteApiMethod, RouteSelection, RuntimeKey,
};
use serde_json::json;

fn chatgpt_web_route() -> ModelRoute {
    ModelRoute {
        model: CHATGPT_WEB_MODEL.to_string(),
        provider: "OpenAI".to_string(),
        api_method: "chatgpt-web".to_string(),
        available: true,
        detail: "logged-in Firefox ChatGPT session".to_string(),
        cheapness: None,
        usage: None,
    }
}

#[test]
fn other_runtime_key_serializes_with_a_named_value() {
    for method in ["chatgpt-web", "custom-transport", "custom/\"quoted\"\\route-λ"] {
        let key = RuntimeKey::from_api_method(
            &ModelRouteApiMethod::Other(method.to_string()),
            "Custom",
        );
        let encoded = serde_json::to_value(&key).expect("other runtime key must serialize");
        assert_eq!(encoded, json!({ "kind": "other", "value": method }));
        let decoded: RuntimeKey = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, key);
        assert_eq!(decoded.stable_id(), method);
    }
}

#[test]
fn other_runtime_key_deserializes_from_its_named_value() {
    let key: RuntimeKey = serde_json::from_value(json!({
        "kind": "other",
        "value": "chatgpt-web"
    }))
    .expect("other runtime key must deserialize");
    assert_eq!(key.stable_id(), "chatgpt-web");
}

#[test]
fn existing_runtime_key_wire_formats_remain_flat() {
    let cases = [
        (RuntimeKey::Current, json!({ "kind": "current" })),
        (RuntimeKey::Copilot, json!({ "kind": "copilot" })),
        (RuntimeKey::OpenRouter, json!({ "kind": "open-router" })),
        (
            RuntimeKey::OpenAiCompatible { profile_id: None },
            json!({ "kind": "open-ai-compatible" }),
        ),
        (
            RuntimeKey::OpenAiCompatible {
                profile_id: Some("custom-profile".to_string()),
            },
            json!({ "kind": "open-ai-compatible", "profile_id": "custom-profile" }),
        ),
    ];
    for (key, expected) in cases {
        assert_eq!(serde_json::to_value(&key).unwrap(), expected);
        assert_eq!(serde_json::from_value::<RuntimeKey>(expected).unwrap(), key);
    }
}

#[test]
fn chatgpt_web_route_selection_round_trips_without_changing_routing() {
    let selection = RouteSelection::from_model_route(&chatgpt_web_route());
    assert_eq!(selection.runtime_key.stable_id(), "chatgpt-web");
    assert_eq!(selection.routed_model_spec(), CHATGPT_WEB_MODEL);

    let encoded = serde_json::to_string(&selection).expect("web route must serialize");
    let decoded: RouteSelection = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, selection);
    assert_eq!(decoded.api_method, "chatgpt-web");
    assert_eq!(decoded.routed_model_spec(), CHATGPT_WEB_MODEL);
}

#[test]
fn chatgpt_web_set_route_request_round_trips() {
    let selection = RouteSelection::from_model_route(&chatgpt_web_route());
    let request = Request::SetRoute {
        id: 42,
        selection: selection.clone(),
    };

    // This is the nested request serialized when the model picker switches routes.
    let encoded = serde_json::to_string(&request).expect("model-switch request must serialize");
    let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(value["type"], "set_route");
    assert_eq!(value["id"], 42);
    assert_eq!(
        value["selection"]["runtime_key"],
        json!({ "kind": "other", "value": "chatgpt-web" })
    );

    match serde_json::from_str::<Request>(&encoded).expect("model-switch request must deserialize") {
        Request::SetRoute {
            id,
            selection: actual,
        } => {
            assert_eq!(id, 42);
            assert_eq!(actual, selection);
            assert_eq!(actual.routed_model_spec(), CHATGPT_WEB_MODEL);
        }
        other => panic!("expected SetRoute, got {other:?}"),
    }
}

#[test]
fn other_runtime_key_rejects_missing_or_non_string_values() {
    for malformed in [
        json!({ "kind": "other" }),
        json!({ "kind": "other", "value": null }),
        json!({ "kind": "other", "value": 42 }),
    ] {
        assert!(serde_json::from_value::<RuntimeKey>(malformed).is_err());
    }
}
