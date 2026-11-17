mod legal_service;

use legal_service::{Deadline, LegalService, MatterIntake, SignedDocument};

fn main() {
    let key = std::env::var("INFRAI_API_KEY").expect("INFRAI_API_KEY is required");
    let service = LegalService::new(key);
    let matter = MatterIntake {
        matter_id: "matter-204".to_string(),
        client_email: "client@example.test".to_string(),
        deadline: Deadline { days_remaining: 3 },
    };
    let document = SignedDocument {
        matter_id: matter.matter_id.clone(),
        download_url: "https://files.example.test/signed/204".to_string(),
    };

    let decision = service.follow_up_decision(&matter, &document);
    println!("matter={} action={decision}", matter.matter_id);
    println!("Infrai account.autorecharge.configure and email.send share one key and base URL");
}

