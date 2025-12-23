use teloxide::{macros::BotCommands, utils::command::BotCommands as _};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "snake_case")]
pub enum Command {
    #[command(description = "Prikaži postojeće komande")]
    Help,
    #[command(description = "Napravi novi zadatak")]
    NoviZadatak,
    #[command(hide)]
    Start,
}

impl Command {
    pub fn get_command_list() -> String {
        let mut commands = String::new();

        for command in Command::bot_commands() {
            commands.push_str(format!("{} - {}\n", command.command, command.description).as_str());
        }

        commands
    }
}
