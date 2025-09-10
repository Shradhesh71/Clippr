use actix_web::{web, HttpResponse, Result};
use serde_json::json;
use uuid::Uuid;
use std::collections::HashMap;

use crate::database::DatabaseManager;
use crate::models::{
    MPCSession, AggSendStep1Request, AggSendStep1Response,
    AggSendStep2Request, AggSendStep2Response,
    AggregateSignaturesBroadcastRequest, AggregateSignaturesBroadcastResponse,
    SignatureShareData
};

pub async fn agg_send_step1(
    data: web::Json<AggSendStep1Request>,
    db: web::Data<DatabaseManager>,
) -> Result<HttpResponse> {
    println!("Starting MPC Step 1 - Commitment Phase");
    
    // Create or get existing session
    let mut session = match db.get_mpc_session(&data.session_id).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            // Create new session
            let participants = vec![data.participant_id.clone()]; // For now, single participant
            let session = MPCSession {
                id: Uuid::new_v4(),
                session_id: data.session_id.clone(),
                user_id: data.user_id.clone(),
                participants,
                current_step: 1,
                commitments: serde_json::json!({}),
                signature_shares: serde_json::json!({}),
                final_signature: None,
                message_to_sign: Some(data.nonce.clone()),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            
            db.create_mpc_session(&session).await.map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!("Failed to create session: {}", e))
            })?;
            
            session
        }
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            })));
        }
    };

    // Validate step
    if session.current_step != 1 {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": format!("Invalid step. Expected step 1, current step: {}", session.current_step)
        })));
    }

    // Generate commitment from nonce
    let commitment = format!("commitment_{}", data.nonce);

    // Store commitment for this participant
    if let serde_json::Value::Object(ref mut commitments) = session.commitments {
        commitments.insert(data.participant_id.clone(), serde_json::Value::String(commitment.clone()));
    }

    // Check if all participants have submitted commitments
    let participants_committed: Vec<String> = if let serde_json::Value::Object(ref commitments) = session.commitments {
        commitments.keys().cloned().collect()
    } else {
        vec![]
    };

    let all_committed = session.participants.len() == participants_committed.len();

    if all_committed {
        // Advance to step 2
        session.current_step = 2;
        session.updated_at = chrono::Utc::now();
    }

    // Update session in database
    db.update_mpc_session(&session).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to update session: {}", e))
    })?;

    let response = AggSendStep1Response {
        session_id: session.session_id.clone(),
        participant_id: data.participant_id.clone(),
        commitment,
        success: true,
        message: "Commitment received successfully".to_string(),
    };

    println!("Step 1 completed for participant: {}", data.participant_id);
    Ok(HttpResponse::Ok().json(response))
}

pub async fn agg_send_step2(
    data: web::Json<AggSendStep2Request>,
    db: web::Data<DatabaseManager>,
) -> Result<HttpResponse> {
    println!("Starting MPC Step 2 - Signature Share Generation");
    
    // Get session
    let mut session = match db.get_mpc_session(&data.session_id).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Session not found"
            })));
        }
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            })));
        }
    };

    // Validate step
    if session.current_step != 2 {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": format!("Invalid step. Expected step 2, current step: {}", session.current_step)
        })));
    }

    // Generate signature share from the message
    let signature_share = format!("sig_share_{}", data.message_to_sign);

    // Store signature share for this participant
    if let serde_json::Value::Object(ref mut shares) = session.signature_shares {
        shares.insert(data.participant_id.clone(), serde_json::Value::String(signature_share.clone()));
    }

    // Check if all participants have submitted signature shares
    let participants_with_shares: Vec<String> = if let serde_json::Value::Object(ref shares) = session.signature_shares {
        shares.keys().cloned().collect()
    } else {
        vec![]
    };

    let all_shares_received = session.participants.len() == participants_with_shares.len();

    if all_shares_received {
        // Ready for aggregation
        session.current_step = 3;
        session.updated_at = chrono::Utc::now();
    }

    // Update session in database
    db.update_mpc_session(&session).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to update session: {}", e))
    })?;

    let response = AggSendStep2Response {
        session_id: session.session_id.clone(),
        participant_id: data.participant_id.clone(),
        signature_share,
        success: true,
        message: "Signature share generated successfully".to_string(),
    };

    println!("Step 2 completed for participant: {}", data.participant_id);
    Ok(HttpResponse::Ok().json(response))
}

pub async fn aggregate_signatures_broadcast(
    data: web::Json<AggregateSignaturesBroadcastRequest>,
    db: web::Data<DatabaseManager>,
) -> Result<HttpResponse> {
    println!("Starting MPC Step 3 - Signature Aggregation and Broadcast");
    
    // Get session
    let mut session = match db.get_mpc_session(&data.session_id).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(json!({
                "error": "Session not found"
            })));
        }
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": format!("Database error: {}", e)
            })));
        }
    };

    // Validate step
    if session.current_step != 3 {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": format!("Invalid step. Expected step 3, current step: {}", session.current_step)
        })));
    }

    // Validate that signature shares are provided
    if data.signature_shares.is_empty() {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "No signature shares provided"
        })));
    }

    // Convert signature shares to HashMap for aggregation
    let mut shares_map = HashMap::new();
    for share_data in &data.signature_shares {
        shares_map.insert(share_data.participant_id.clone(), share_data.signature_share.clone());
    }

    // Perform signature aggregation
    let aggregated_signature = simulate_signature_aggregation(&shares_map, &data.message_to_sign);

    // Store final signature
    session.final_signature = Some(aggregated_signature.clone());
    session.updated_at = chrono::Utc::now();

    // Update session in database
    db.update_mpc_session(&session).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to update session: {}", e))
    })?;

    // Generate a dummy public key for now
    let public_key = "dummy_public_key_placeholder".to_string();

    let response = AggregateSignaturesBroadcastResponse {
        session_id: session.session_id.clone(),
        final_signature: aggregated_signature.clone(),
        public_key,
        success: true,
        message: "Signature aggregated successfully".to_string(),
    };

    println!("MPC Protocol completed successfully for session: {}", session.session_id);
    Ok(HttpResponse::Ok().json(response))
}

// Simulate signature aggregation for demonstration
fn simulate_signature_aggregation(signature_shares: &HashMap<String, String>, message: &str) -> String {
    use sha2::{Sha256, Digest};
    
    // Combine all signature shares with the message
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    
    // Add each signature share to the hash
    let mut sorted_shares: Vec<_> = signature_shares.iter().collect();
    sorted_shares.sort_by_key(|(k, _)| *k);
    
    for (participant, share) in sorted_shares {
        hasher.update(participant.as_bytes());
        hasher.update(share.as_bytes());
    }
    
    let result = hasher.finalize();
    hex::encode(result)
}
