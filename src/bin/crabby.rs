use clap::{crate_authors, crate_version, Clap};
use futures::{future, stream::TryStreamExt};
use itertools::Itertools;
use octocrab::Octocrab;
use octocrabby::{block_user, check_follow, cli, models::UserInfo, parse_repo_path, pull_requests};
use std::collections::{HashMap, HashSet};

type Void = Result<(), Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Void {
    let opts: Opts = Opts::parse();
    let _ = cli::init_logging(opts.verbose);
    let instance = octocrabby::init(opts.token)?;

    match opts.command {
        Command::BlockUsers => {
            // Note that only the first field is used, and is expected to be a GitHub login username
            let mut reader = csv::Reader::from_reader(std::io::stdin());
            let mut usernames = vec![];

            for record in reader.records() {
                usernames.push(record?.get(0).unwrap().to_string());
            }

            for username in usernames {
                if block_user(&instance, &username).await? {
                    log::info!("Successfully blocked {}", username)
                } else {
                    log::warn!("{} was already blocked", username)
                };
            }
        }
        Command::ListFollowers => {
            octocrabby::get_followers(&instance)
                .try_for_each(|user| {
                    println!("{},{}", user.login, user.id);
                    future::ok(())
                })
                .await?
        }
        Command::ListFollowing => {
            octocrabby::get_following(&instance)
                .try_for_each(|user| {
                    println!("{},{}", user.login, user.id);
                    future::ok(())
                })
                .await?
        }
        Command::ListBlocks => {
            octocrabby::get_blocks(&instance)
                .try_for_each(|user| {
                    println!("{},{}", user.login, user.id);
                    future::ok(())
                })
                .await?
        }
        Command::ListPrContributors { repo_path } => {
            if let Some((owner, repo)) = parse_repo_path(&repo_path) {
                log::info!("Loading pull requests");
                let mut prs = pull_requests(&instance, owner, repo)
                    .try_collect::<Vec<_>>()
                    .await?;
                prs.sort_unstable_by_key(|pr| pr.user.login.clone());

                let by_username = prs
                    .into_iter()
                    .group_by(|pr| (pr.user.login.clone(), pr.user.id));

                let results = by_username
                    .into_iter()
                    .map(|((username, user_id), prs)| {
                        let batch = prs.collect::<Vec<_>>();
                        (
                            username,
                            user_id,
                            batch.len(),
                            batch.into_iter().map(|pr| pr.created_at).min().unwrap(),
                        )
                    })
                    .collect::<Vec<_>>();

                let usernames = results
                    .iter()
                    .map(|(username, _, _, _)| username.clone())
                    .collect::<Vec<_>>();

                // Load additional information that's only available if you're authenticated
                let mut additional_info: Option<AdditionalUserInfo> =
                    if instance.current().user().await.is_ok() {
                        Some(load_additional_user_info(&instance, &usernames).await?)
                    } else {
                        None
                    };

                let mut writer = csv::Writer::from_writer(std::io::stdout());

                for (username, user_id, pr_count, first_pr_date) in results {
                    let mut record =
                        vec![username.clone(), user_id.to_string(), pr_count.to_string()];

                    // Add other fields to the record if you're authenticated
                    if let Some(AdditionalUserInfo {
                        ref follows_you,
                        ref you_follow,
                        ref mut user_info,
                    }) = additional_info
                    {
                        let (age, name, twitter_username) = match user_info.remove(&username) {
                            Some(info) => (
                                (first_pr_date - info.created_at).num_days(),
                                info.name.unwrap_or_default(),
                                info.twitter_username.unwrap_or_default(),
                            ),
                            None => {
                                // These values will be used for accounts such as dependabot
                                (-1, "".to_string(), "".to_string())
                            }
                        };

                        record.push(age.to_string());
                        record.push(name);
                        record.push(you_follow.contains(&username).to_string());
                        record.push(follows_you.contains(&username).to_string());
                        record.push(twitter_username);
                    };

                    writer.write_record(&record)?;
                }
            } else {
                log::error!("Invalid repository path: {}", repo_path);
            }
        }
        Command::CheckFollow { user, follower } => {
            let target_user = match user {
                Some(value) => value,
                None => instance.current().user().await?.login,
            };

            let result = check_follow(&instance, &follower, &target_user).await?;

            println!("{}", result);
        }
    }

    Ok(())
}

#[derive(Clap)]
#[clap(name = "crabby", version = crate_version!(), author = crate_authors!())]
struct Opts {
    /// A GitHub personal access token (not needed for all operations)
    #[clap(short, long)]
    token: Option<String>,
    #[clap(short, long, parse(from_occurrences))]
    /// Logging verbosity
    verbose: i32,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Clap)]
enum Command {
    /// Block a list of users provided in CSV format to stdin
    BlockUsers,
    /// List the authenticated user's followers in CSV format to stdout
    ListFollowers,
    /// List accounts the authenticated user follows in CSV format to stdout
    ListFollowing,
    /// List accounts the authenticated user blocks in CSV format to stdout
    ListBlocks,
    /// List PR contributors for the given repository
    ListPrContributors {
        /// The repository to check for pull requests
        #[clap(short, long)]
        repo_path: String,
    },
    /// Check whether one user follows another
    CheckFollow {
        /// The possibly followed user
        #[clap(short, long)]
        user: Option<String>,
        /// The possible follower
        #[clap(short, long)]
        follower: String,
    },
}

struct AdditionalUserInfo {
    follows_you: HashSet<String>,
    you_follow: HashSet<String>,
    user_info: HashMap<String, UserInfo>,
}

async fn load_additional_user_info(
    instance: &Octocrab,
    usernames: &[String],
) -> octocrab::Result<AdditionalUserInfo> {
    log::info!("Loading follower information");
    let follows_you = octocrabby::get_followers(&instance)
        .and_then(|user| future::ok(user.login))
        .try_collect()
        .await?;

    log::info!("Loading following information");
    let you_follow = octocrabby::get_following(&instance)
        .and_then(|user| future::ok(user.login))
        .try_collect()
        .await?;

    log::info!(
        "Loading additional user information for {} users",
        usernames.len()
    );
    let user_info: HashMap<String, UserInfo> = octocrabby::get_users_info(&instance, &usernames)
        .await?
        .into_iter()
        .map(|info| (info.login.clone(), info))
        .collect();

    Ok(AdditionalUserInfo {
        follows_you,
        you_follow,
        user_info,
    })
}
