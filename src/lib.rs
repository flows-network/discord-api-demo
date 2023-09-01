use discord_flows::{
    application_command_handler,
    http::HttpBuilder,
    message_handler,
    model::{
        application::interaction::InteractionResponseType,
        application_command::CommandDataOptionValue,
        prelude::application::interaction::application_command::ApplicationCommandInteraction,
        Message,
    },
    Bot, ProvidedBot,
};
use flowsnet_platform_sdk::logger;
use http_req::{
    request::{Method, Request},
    uri::Uri,
};
use serde::Deserialize;
use serde_json;
use std::env;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    logger::init();
    let discord_token = env::var("discord_token").unwrap();
    let bot = ProvidedBot::new(&discord_token);

    register_commands().await;

    bot.listen_to_messages().await;

    let channel_id = env::var("discord_channel_id").unwrap_or("channel_id not found".to_string());
    let channel_id = channel_id.parse::<u64>().unwrap();
    bot.listen_to_application_commands_from_channel(channel_id)
        .await;
}

#[message_handler]
async fn handle(msg: Message) {
    logger::init();
    let discord_token = std::env::var("discord_token").unwrap();
    let bot = ProvidedBot::new(&discord_token);

    if msg.author.bot {
        return;
    }
    let client = bot.get_client();
    _ = client
        .send_message(
            msg.channel_id.into(),
            &serde_json::json!({
                "content": msg.content,
            }),
        )
        .await;
}

#[application_command_handler]
async fn handler(ac: ApplicationCommandInteraction) {
    logger::init();
    let discord_token = env::var("discord_token").unwrap();
    let bot = ProvidedBot::new(discord_token);
    let client = bot.get_client();

    client.set_application_id(ac.application_id.into());

    _ = client
        .create_interaction_response(
            ac.id.into(),
            &ac.token,
            &serde_json::json!({
                "type": InteractionResponseType::DeferredChannelMessageWithSource as u8,
            }),
        )
        .await;
    let options = &ac.data.options;

    match ac.data.name.as_str() {
        "test" => {
            let city = match options
                .get(0)
                .expect("Expected city option")
                .resolved
                .as_ref()
                .expect("Expected city object")
            {
                CommandDataOptionValue::String(s) => s,
                _ => panic!("Expected string for city"),
            };

            let resp_inner = match get_weather(&city) {
                Some(w) => format!(
                    r#"Today: {},
                Low temperature: {} °C,
                High temperature: {} °C,
                Wind Speed: {} km/h"#,
                    w.weather
                        .first()
                        .unwrap_or(&Weather {
                            main: "Unknown".to_string()
                        })
                        .main,
                    w.main.temp_min as i32,
                    w.main.temp_max as i32,
                    w.wind.speed as i32
                ),
                None => String::from("No city or incorrect spelling"),
            };
            let resp = serde_json::json!({ "content": resp_inner });
            _ = client
                .edit_original_interaction_response(&ac.token, &resp)
                .await;
        }
        _ => {}
    }
}

#[derive(Deserialize, Debug)]
struct ApiResult {
    weather: Vec<Weather>,
    main: Main,
    wind: Wind,
}

#[derive(Deserialize, Debug)]
struct Weather {
    main: String,
}

#[derive(Deserialize, Debug)]
struct Main {
    temp_max: f64,
    temp_min: f64,
}

#[derive(Deserialize, Debug)]
struct Wind {
    speed: f64,
}

fn get_weather(city: &str) -> Option<ApiResult> {
    let mut writer = Vec::new();
    let api_key = env::var("API_KEY").unwrap_or("fake_api_key".to_string());
    let query_str = format!(
        "https://api.openweathermap.org/data/2.5/weather?q={city}&units=metric&appid={api_key}"
    );

    let uri = Uri::try_from(query_str.as_str()).unwrap();
    match Request::new(&uri).method(Method::GET).send(&mut writer) {
        Err(_e) => log::error!("Error getting response from weather api: {:?}", _e),

        Ok(res) => {
            if !res.status_code().is_success() {
                log::error!("weather api http error: {:?}", res.status_code());
                return None;
            }
            match serde_json::from_slice::<ApiResult>(&writer) {
                Err(_e) => log::error!("Error deserializing weather api response: {:?}", _e),
                Ok(w) => {
                    log::info!("Weather: {:?}", w);
                    return Some(w);
                }
            }
        }
    };
    None
}

async fn register_commands() {

    let bot_id = env::var("bot_id").unwrap_or("1124137839601406013".to_string());
 
    let command = serde_json::json!({
        "name": "test",
        "description": "Get the weather for a city",
        "options": [
            {
                "name": "city",
                "description": "The city to lookup",
                "type": 3,
                "required": true
            }
        ]
    });

    let discord_token = env::var("discord_token").unwrap();
    let http_client = HttpBuilder::new(discord_token)
        .application_id(bot_id.parse().unwrap())
        .build();

    match http_client
        .create_global_application_command(&command)
        .await
    {
        Ok(_) => log::info!("Successfully registered command"),
        Err(err) => log::error!("Error registering command: {}", err),
    }
}
