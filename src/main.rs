#![allow(dead_code)]
#![allow(non_snake_case)]
use crate::egui::Vec2;
use eframe::egui;
use reqwest::Error;
use rusqlite::{Connection, Result};
use serde::Deserialize;
use std::{thread, time};

#[derive(Debug)]
enum SteamAPIAppError {
    RusqliteError(rusqlite::Result<()>),
    EFrameError(eframe::Result<()>),
    Error,
}

impl SteamAPIAppError {}

impl From<rusqlite::Error> for SteamAPIAppError {
    fn from(err: rusqlite::Error) -> SteamAPIAppError {
        SteamAPIAppError::RusqliteError(Err(err))
    }
}

impl From<reqwest::Error> for SteamAPIAppError {
    fn from(_err: reqwest::Error) -> SteamAPIAppError {
        SteamAPIAppError::Error
    }
}

#[derive(Deserialize, Debug)]
struct Player {
    steamid: String,
    personaname: String,
}

#[derive(Deserialize, Debug)]
struct SteamResponse {
    players: Vec<Player>,
}

#[derive(Deserialize, Debug)]
struct SteamJson {
    response: SteamResponse,
}

#[derive(Deserialize, Debug, Default)]
struct Stat {
    name: String,
    value: i32,
}

#[derive(Deserialize, Debug, Default)]
struct PlayerStat {
    steamID: String,
    stats: Vec<Stat>,
}

#[derive(Deserialize, Debug, Default)]
struct SteamJson2 {
    // steamID1: String,
    playerstats: PlayerStat,
}

#[derive(Deserialize, Debug, Default)]
struct Friend {
    steamid: String,
}

#[derive(Deserialize, Debug, Default)]
struct FriendsList {
    friends: Vec<Friend>,
}

#[derive(Deserialize, Debug, Default)]
struct SteamJson3 {
    // steamID1: String,
    friendslist: FriendsList,
}

async fn get_player_summaries(key: &str, steamids: &str) -> Result<SteamJson, Error> {
    let request_player_summaries = format!(
        "http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={key}&steamids={id}",
        key = key,
        id = steamids,
    );
    // println!("{}", request_player_summaries);
    let client = reqwest::Client::new();
    let response = client
        .get(&request_player_summaries)
        .header(reqwest::header::USER_AGENT, "n00bs testing_application")
        .send()
        .await?;
    // println!("{:?}", response);

    let response_object: SteamJson = response.json().await?;
    Ok(response_object)
}

async fn get_user_stats_for_game(key: &str, steamids: &str) -> Result<SteamJson2, Error> {
    let request_player_summaries = format!(
        "http://api.steampowered.com/ISteamUserStats/GetUserStatsForGame/v0002/?appid=440&key={key}&steamid={id}",
        key = key,
        id = steamids,
    );
    // println!("request_player_summaries: {}", request_player_summaries);
    let client = reqwest::Client::new();
    let response = client
        .get(&request_player_summaries)
        .header(reqwest::header::USER_AGENT, "n00bs testing_application")
        .send()
        .await?;
    // println!("my response: {:?}", response);

    let response_object: SteamJson2 = response.json().await?;
    Ok(response_object)
}

async fn get_user_friends(key: &str, steamids: &str) -> Result<SteamJson3, Error> {
    let request_player_friends = format!(
        "http://api.steampowered.com/ISteamUser/GetFriendList/v0001/?key={key}&steamid={id}&relationship=friend",
        key = key,
        id = steamids,
    );
    // println!("request_player_friends: {}", request_player_friends);
    let client = reqwest::Client::new();
    let response = client
        .get(&request_player_friends)
        .header(reqwest::header::USER_AGENT, "n00bs testing_application")
        .send()
        .await?;
    // println!("my response: {:?}", response);

    let response_object: SteamJson3 = response.json().await?;
    Ok(response_object)
}

fn get_checked_profiles_count(conn: &mut Connection) -> Result<i32, SteamAPIAppError> {
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM steamids_checked")?;
    let number_processed_profiles = stmt.query_map([], |row| row.get(0))?;
    let mut no_prof: Vec<i32> = Vec::new();
    for prof in number_processed_profiles {
        no_prof.push(prof?);
    }
    Ok(no_prof[0])
}

fn get_unchecked_profiles_count(conn: &mut Connection) -> Result<i32, SteamAPIAppError> {
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM steamids_unchecked")?;
    let unprocessed_profiles = stmt.query_map([], |row| row.get(0))?;
    let mut profs: Vec<i32> = Vec::new();
    for profile in unprocessed_profiles {
        profs.push(profile?);
    }
    Ok(profs[0])
}

fn get_unchecked_profiles_last_row_id(conn: &mut Connection) -> Result<i32, SteamAPIAppError> {
    let mut stmt = conn.prepare("SELECT id FROM steamids_unchecked ORDER BY id DESC LIMIT 1")?;
    let last_row = stmt.query_map([], |row| row.get(0))?;
    let mut rows: Vec<i32> = Vec::new();
    for row in last_row {
        rows.push(row?);
    }
    if rows.len() == 0 {
        println!("empty checked");
        return Ok(0);
    }
    Ok(rows[0])
}

fn get_checked_profiles_last_row_id(conn: &mut Connection) -> Result<i32, SteamAPIAppError> {
    let mut stmt = conn.prepare("SELECT id FROM steamids_checked ORDER BY id DESC LIMIT 1")?;
    let last_row = stmt.query_map([], |row| row.get(0))?;
    let mut rows: Vec<i32> = Vec::new();
    for row in last_row {
        rows.push(row?);
    }
    if rows.len() == 0 {
        println!("empty checked");
        return Ok(0);
    }
    Ok(rows[0])
}

fn get_unchecked_profile(conn: &mut Connection) -> Result<String, SteamAPIAppError> {
    let mut stmt = conn.prepare("SELECT steamid FROM steamids_unchecked LIMIT 1")?;
    let unprocessed_profiles = stmt.query_map([], |row| row.get(0))?;
    let mut profs: Vec<String> = Vec::new();
    for profile in unprocessed_profiles {
        profs.push(profile?);
    }
    Ok(profs[0].clone())
}

fn get_if_already_checked(
    conn: &mut Connection,
    steamid: String,
) -> Result<bool, SteamAPIAppError> {
    let mut stmt = conn.prepare("SELECT steamid FROM steamids_checked WHERE steamid=:steamid")?;
    let exists_profile = stmt.query_map(&[(":steamid", &steamid)], |row| row.get(0))?;
    let mut profs: Vec<String> = Vec::new();
    for profile in exists_profile {
        profs.push(profile?);
    }
    if profs.len() > 0 {
        println!("duped!!!");
        return Ok(true);
    }
    Ok(false)
}

fn delete_unchecked_profile(
    conn: &mut Connection,
    profile: String,
) -> Result<(), SteamAPIAppError> {
    conn.execute(
        "DELETE FROM steamids_unchecked WHERE steamid = (?1)",
        &[&profile],
    )?;
    Ok(())
}

#[derive(Default)]
struct EframeExampleApp {
    steam_key_text: String,
    initial_profile_id: String,
    run_limit: i32,
    enabled_button: bool,
}

impl EframeExampleApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        EframeExampleApp {
            enabled_button: false,
            ..Default::default()
        }
    }
}

impl eframe::App for EframeExampleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("TF2 Stat Collector");
                ui.horizontal(|ui| {
                    ui.label("Enter Steam Key: ");
                    ui.text_edit_singleline(&mut self.steam_key_text);
                });
                if self.steam_key_text.is_empty() {
                    ui.label("Please enter steam key first, before running");
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Enter Profile ID: ");
                    ui.text_edit_singleline(&mut self.initial_profile_id);
                });
                if self.initial_profile_id.is_empty() {
                    ui.label("Please enter initial profile ID, before running");
                }
                ui.separator();
                if !self.steam_key_text.is_empty() && !self.initial_profile_id.is_empty() {
                    self.enabled_button = true;
                }
                ui.add(egui::Slider::new(&mut self.run_limit, 1..=10000).text("limits"));
                if self.enabled_button {
                    if ui.button("Run").clicked() {
                        self.enabled_button = false;
                        let temp_string = self.steam_key_text.clone();
                        let temp_initial_id = self.initial_profile_id.clone();
                        let temp_limit = self.run_limit;
                        tokio::spawn(async move {
                            match run_it(temp_string, temp_initial_id, temp_limit).await {
                                Ok(response) => response,
                                Err(error) => panic!("my errors: {:?}", error),
                            };
                        });
                    }
                } else if ui.add_enabled(false, egui::Button::new("Run")).clicked() {
                }
            });
        });
    }
}

async fn run_it(
    steam_key: String,
    initial_profile_id: String,
    run_limit: i32,
) -> Result<(), SteamAPIAppError> {
    let class_time_played = [
        "Scout.accum.iPlayTime",
        "Soldier.accum.iPlayTime",
        "Spy.accum.iPlayTime",
        "Pyro.accum.iPlayTime",
        "Medic.accum.iPlayTime",
        "Demoman.accum.iPlayTime",
        "Heavy.accum.iPlayTime",
        "Engineer.accum.iPlayTime",
        "Sniper.accum.iPlayTime",
    ];
    // println!("{:?}", get_player_summaries(steam_key, profile_id).await?);
    // println!("run_limit: {}", run_limit);
    let mut conn = Connection::open("steam_info.db")?;

    let count = get_checked_profiles_count(&mut conn)?;
    if count == 0 {
        conn.execute(
            "INSERT INTO steamids_unchecked (id, steamid) VALUES (?1, ?2)",
            (1, &initial_profile_id.as_str()),
        )?;
    }
    let mut dup_number = 0;
    for _i in 0..run_limit {
        // pause for steam API politeness
        let pause_time = time::Duration::from_millis(1400);
        thread::sleep(pause_time);
        // get profile that needs processing
        if get_unchecked_profiles_last_row_id(&mut conn)? == 0 {
            break;
        }
        let profile = get_unchecked_profile(&mut conn)?;

        let already_checked = get_if_already_checked(&mut conn, profile.clone())?;
        if already_checked {
            dup_number = dup_number + 1;
            println!("dup number: {}", dup_number);
            delete_unchecked_profile(&mut conn, profile.clone())?;
            continue;
        }

        // // get the friends of current profile
        // let friends_list = match get_user_friends(steam_key.as_str(), profile.as_str()).await {
        //     Ok(response) => response,
        //     Err(_error) => {
        //         delete_unchecked_profile(&mut conn, profile.clone())?;
        //         continue;
        //     }
        // };
        // // need to keep indexing unique
        // let mut unchecked_count = get_unchecked_profiles_last_row_id(&mut conn)?;
        // for friend in &friends_list.friendslist.friends {
        //     conn.execute(
        //         "INSERT INTO steamids_unchecked (id, steamid) VALUES (?1, ?2)",
        //         (&unchecked_count + 1, &friend.steamid),
        //     )?;
        //     unchecked_count = unchecked_count + 1;
        // }
        // insert into checked, to keep track of already processed profiles
        // also for foreign key to TF2stats
        let checked_count = get_checked_profiles_last_row_id(&mut conn)?;
        conn.execute(
            "INSERT INTO steamids_checked (id, steamid) VALUES (?1, ?2)",
            (checked_count + 1, &profile.as_str()),
        )?;
        // get the stats for TF2 played time
        let response_object =
            match get_user_stats_for_game(steam_key.as_str(), profile.as_str()).await {
                Ok(response) => response,
                Err(_error) => {
                    delete_unchecked_profile(&mut conn, profile.clone())?;
                    continue;
                }
            };
        // process the stats and insert into table
        for stat in &response_object.playerstats.stats {
            if class_time_played.iter().any(|&x| x == stat.name) {
                // println!("Stat: {}, value: {}", stat.name, stat.value); //TODO Remove
                conn.execute(
                    "INSERT INTO TF2stats (name, value, steamid_id) VALUES (?1, ?2, ?3)",
                    (&stat.name, &stat.value, checked_count + 1),
                )?;
            }
        }
        // processed a profile, remove current from steamids_unchecked
        delete_unchecked_profile(&mut conn, profile.clone())?;
    }
    println!("DONE!!!!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), SteamAPIAppError> {
    // create the database if it doesn't exist
    let conn = Connection::open("steam_info.db")?;

    conn.execute(
        "CREATE TABLE if not exists steamids_unchecked (
            id   INTEGER PRIMARY KEY,
            steamid TEXT NOT NULL
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE if not exists steamids_checked (
            id   INTEGER PRIMARY KEY,
            steamid TEXT NOT NULL
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE if not exists TF2stats (
            id   INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            value INTEGER,
            steamid_id INTEGER NOT NULL,
            FOREIGN KEY (steamid_id)
                REFERENCES steamids_checked (id)
        )",
        (),
    )?;
    conn.close().unwrap();
    let options = eframe::NativeOptions {
        initial_window_size: Some(Vec2 { x: 930.0, y: 650.0 }),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "TF2 Stat Collector",
        options,
        Box::new(|cc| Box::new(EframeExampleApp::new(cc))),
    )
    .map_err(|e| SteamAPIAppError::EFrameError(Err(e)))
}
