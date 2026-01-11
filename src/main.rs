use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::path::Path;

// 1. The Config File Structure (Works for JSON, TOML, and YAML)
#[derive(Deserialize, Debug)]
struct ConfigFile {
    api_key: Option<String>,
    postal_code: Option<String>,
    species: Option<String>,
    miles: Option<u32>,
}

// 2. The CLI Arguments
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// The zip code to search in (Optional on CLI if set in config)
    postal_code: Option<String>,

    /// Search radius in miles
    #[arg(short, long)]
    miles: Option<u32>,

    /// Species to search for
    #[arg(short, long)]
    species: Option<String>,

    /// API Key
    #[arg(long, env = "RESCUE_GROUPS_API_KEY", hide_env_values = true)]
    api_key: Option<String>,

    /// Path to config file (supports .toml, .yaml, .json)
    #[arg(long, default_value = "config.toml")]
    config: String,
}

// 3. Merged Settings
struct Settings {
    api_key: String,
    postal_code: String,
    miles: u32,
    species: String,
}

// Helper to merge CLI, Env, and File
fn merge_configuration(cli: Cli) -> Result<Settings, Box<dyn Error>> {
    let config_path = Path::new(&cli.config);
    
    // A. Load config based on extension
    let file_config: Option<ConfigFile> = if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        let ext = config_path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default(); // FIX: Changed from unwrap_or_else(|| "".to_string())

        match ext.as_str() {
            "toml" => Some(toml::from_str(&content)?),
            "json" => Some(serde_json::from_str(&content)?),
            "yaml" | "yml" => Some(serde_yaml::from_str(&content)?),
            _ => return Err(format!("Unsupported config format: .{}", ext).into()),
        }
    } else {
        // Only error if the user explicitly provided a custom path that doesn't exist.
        // If it's the default "config.toml" and it's missing, just ignore it.
        if cli.config != "config.toml" {
             return Err(format!("Config file not found: {}", cli.config).into());
        }
        None
    };

    // B. Priority: CLI -> Config File -> Default
    let miles = cli.miles
        .or(file_config.as_ref().and_then(|c| c.miles))
        .unwrap_or(50);

    let species = cli.species
        .or(file_config.as_ref().and_then(|c| c.species.clone()))
        .unwrap_or_else(|| "dogs".to_string());

    let api_key = cli.api_key
        .or(file_config.as_ref().and_then(|c| c.api_key.clone()))
        .ok_or("API Key is missing! Provide it via --api-key, RESCUE_GROUPS_API_KEY, or config file")?;

    let postal_code = cli.postal_code
        .or(file_config.as_ref().and_then(|c| c.postal_code.clone()))
        .ok_or("Postal Code is missing! Provide it via argument or config file")?;

    Ok(Settings {
        api_key,
        postal_code,
        miles,
        species,
    })
}

async fn fetch_pets(settings: &Settings) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    
    let url = format!(
        "https://api.rescuegroups.org/v5/public/animals/search/available/{}/haspic",
        settings.species
    );

    let body = json!({
        "data": {
            "filterRadius": {
                "miles": settings.miles,
                "postalcode": settings.postal_code
            }
        }
    });

    let response = client
        .post(&url)
        .header("Authorization", &settings.api_key)
        .header("Content-Type", "application/vnd.api+json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("API Request Failed: Status {}", response.status()).into());
    }

    let data: serde_json::Value = response.json().await?;
    
    let animals = data.get("data")
        .and_then(|d| d.as_array())
        .ok_or("No 'data' field found in API response")?;

    if animals.is_empty() {
        return Ok(format!("No adoptable {} found within {} miles of {}.", settings.species, settings.miles, settings.postal_code));
    }

    let formatted_results: Vec<String> = animals.iter().map(|animal| {
        let attrs = &animal["attributes"];
        
        let name = attrs["name"].as_str().unwrap_or("Unknown Name");
        let breed = attrs["breedString"].as_str().unwrap_or("Unknown Breed");
        let sex = attrs["sex"].as_str().unwrap_or("Unknown Sex");
        let dist = attrs["distance"].as_u64().unwrap_or(0);
        let profile_url = attrs["url"].as_str().unwrap_or("#");

        let image_markdown = attrs["orgsAnimalsPictures"]
            .as_array()
            .and_then(|pics| pics.first())
            .and_then(|p| p["urlSecureFullsize"].as_str())
            .map(|img_url| format!("![{}]({})", name, img_url))
            .unwrap_or_else(|| "(No Image Available)".to_string());

        format!(
            "### [{name}]({url})\n\
             **Breed:** {breed} ({sex})\n\
             **Distance:** {dist} miles away\n\n\
             {img}\n", 
            name = name,
            url = profile_url,
            breed = breed,
            sex = sex,
            dist = dist,
            img = image_markdown
        )
    }).collect();

    Ok(formatted_results.join("\n---\n"))
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match merge_configuration(cli) {
        Ok(settings) => {
            println!("Searching for {} near {}...", settings.species, settings.postal_code);
            match fetch_pets(&settings).await {
                Ok(output) => println!("{}", output),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Err(e) => {
            eprintln!("Configuration Error: {}", e);
            std::process::exit(1);
        }
    }
}
