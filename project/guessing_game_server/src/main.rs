use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Game state ────────────────────────────────────────────────────────────────

struct AppState {
    games: Mutex<HashMap<String, Game>>,
}

struct Game {
    secret: u32,
    attempts: u32,
    low: u32,
    high: u32,
    won: bool,
}

impl Game {
    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        Game {
            secret: (seed % 1000) + 1,
            attempts: 0,
            low: 1,
            high: 1000,
            won: false,
        }
    }
}

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct GuessRequest {
    game_id: String,
    guess: u32,
}

#[derive(Serialize)]
struct NewGameResponse {
    game_id: String,
}

#[derive(Serialize)]
struct GuessResponse {
    result: String,   // "low" | "high" | "win" | "error"
    message: String,
    attempts: u32,
    low: u32,
    high: u32,
    won: bool,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn new_game(data: web::Data<AppState>) -> impl Responder {
    let mut games = data.games.lock().unwrap();
    let game_id = uuid::Uuid::new_v4().to_string();
    games.insert(game_id.clone(), Game::new());
    HttpResponse::Ok().json(NewGameResponse { game_id })
}

async fn guess(
    data: web::Data<AppState>,
    body: web::Json<GuessRequest>,
) -> impl Responder {
    let mut games = data.games.lock().unwrap();

    let game = match games.get_mut(&body.game_id) {
        Some(g) => g,
        None => {
            return HttpResponse::NotFound().json(GuessResponse {
                result: "error".into(),
                message: "Game not found. Start a new game.".into(),
                attempts: 0,
                low: 1,
                high: 1000,
                won: false,
            });
        }
    };

    if game.won {
        return HttpResponse::Ok().json(GuessResponse {
            result: "error".into(),
            message: "Game already won. Start a new game.".into(),
            attempts: game.attempts,
            low: game.low,
            high: game.high,
            won: true,
        });
    }

    if body.guess < 1 || body.guess > 1000 {
        return HttpResponse::BadRequest().json(GuessResponse {
            result: "error".into(),
            message: "Please enter a number between 1 and 1000.".into(),
            attempts: game.attempts,
            low: game.low,
            high: game.high,
            won: false,
        });
    }

    game.attempts += 1;

    let response = match body.guess.cmp(&game.secret) {
        std::cmp::Ordering::Less => {
            game.low = game.low.max(body.guess + 1);
            GuessResponse {
                result: "low".into(),
                message: format!("{} is too low — go higher", body.guess),
                attempts: game.attempts,
                low: game.low,
                high: game.high,
                won: false,
            }
        }
        std::cmp::Ordering::Greater => {
            game.high = game.high.min(body.guess - 1);
            GuessResponse {
                result: "high".into(),
                message: format!("{} is too high — go lower", body.guess),
                attempts: game.attempts,
                low: game.low,
                high: game.high,
                won: false,
            }
        }
        std::cmp::Ordering::Equal => {
            game.won = true;
            let attempts = game.attempts;
            GuessResponse {
                result: "win".into(),
                message: format!(
                    "You got it! The number was {}. You needed {} guess{}.",
                    game.secret,
                    attempts,
                    if attempts == 1 { "" } else { "es" }
                ),
                attempts,
                low: game.low,
                high: game.high,
                won: true,
            }
        }
    };

    HttpResponse::Ok().json(response)
}

async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("index.html"))
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        games: Mutex::new(HashMap::new()),
    });

    println!("Server running at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/", web::get().to(index))
            .route("/api/new", web::post().to(new_game))
            .route("/api/guess", web::post().to(guess))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
