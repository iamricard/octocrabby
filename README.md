# GitHub block list management

[![Build status](https://img.shields.io/github/workflow/status/travisbrown/octocrabby/ci.svg)](https://github.com/travisbrown/octocrabby/actions)

Octocrabby is a small set of command-line tools and [Octocrab][octocrab] extensions
that are focused on managing block lists on [GitHub][github].
This project [was inspired][1375333996398325762] by an [open letter][rms-support-letter]
supporting Richard Stallman, which has been signed by several thousand GitHub users I
don't want to accidentally donate free open source support to.

This project may eventually get merged into [cancel-culture][cancel-culture], which is currently
focused on archiving and block list management for Twitter.

## Usage

This project is made of [Rust][rust], and you currently need Rust and [Cargo][cargo] installed
to use it. If you've followed [these instructions][rust-installation] and cloned this repo locally,
you can build the CLI by running the following command from the project directory:

```bash
$ cargo build --release
   Compiling bytes v1.0.1
   ...
   Compiling octocrabby v0.1.0 (/home/travis/projects/octocrabby)
    Finished release [optimized] target(s) in 1m 35s
```

Most operations require a [GitHub personal access token][github-token], which you currently have to
provide as a command-line option. If you want to use the mass-blocking functionality, you'll need to
select the `user` scope when creating your token. If you only want to generate reports or export your
follower or block lists, that shouldn't be necessary.

### Contributor reports

One operation that doesn't require a personal access token is `list-pr-contributors`:

```bash
$ target/release/crabby -vvvv list-pr-contributors -r rms-support-letter/rms-support-letter.github.io
```

If no token is provided, this command will output a CSV document with a row for each GitHub user who contributed
a pull request to the given repository. Each row will have three columns:

1. GitHub username
2. GitHub user ID
3. Number of PRs for this repository

For example:

```csv
0312birdzhang,1762041,1
0hueliSJWpidorasi,81465353,1
0kalekale,31927746,1
0ver3inker,53104897,1
0x0000ff,1977210,1
```

If you provide a personal access token to this command (via `-t`), the output will include several additional columns:

1. GitHub username
2. GitHub user ID
3. Number of PRs for this repo
4. Number of days between account creation and the first PR to this repo
5. The user's name (if available)
6. A boolean indicating whether you follow this user
7. A boolean indicating whether this user follows you

For example:

```csv
0312birdzhang,1762041,1,3229,BirdZhang,false,false
0hueliSJWpidorasi,81465353,1,0,,false,false
0kalekale,31927746,1,1288,kalekale,false,false
0ver3inker,53104897,1,617,0ver3inker,false,false
0x0000ff,1977210,1,3176,,false,false
```

You can find copies of the output of this command in this project's [data directory][data-directory].

This allows us to see how many of the signatories were using single-purpose throwaway accounts, for example.
As of this morning, only 82 of the 3,000+ accounts were created on the same day they opened their PR:

```bash
$ awk -F, '$4 == 0' rms-contributors.csv | wc
     82     102    3282
```

You can also check how many of the signers follow you on GitHub:

```bash
$ egrep -r "true,(true|false)$" rms-contributors.csv | wc
      0       0       0
```

And how many you follow:

```bash
$ egrep -r "true$" rms-contributors.csv | wc
      0       0       0
```

Good.

### Follow and block list export

The CLI also allows you to export lists of users you follow, are followed by, and block:

```bash
$ target/release/crabby -vvvv -t $GH_TOKEN list-following | wc
     24      24     408

$ target/release/crabby -vvvv -t $GH_TOKEN list-followers | wc
    575     575   10416

$ target/release/crabby -vvvv -t $GH_TOKEN list-blocks | head
alexy,27491
soc,42493
jdegoes,156745
vmarquez,427578
gvolpe,443978
neko-kai,450507
hmemcpy,601206
kubukoz,894884
propensive,1024588
phderome,11035032
```

The format is a two-column CSV with username and user ID.

In general it's probably a good idea to save the output of the `list-blocks` command before using
the mass-blocking functionality in the next section.

### Mass blocking

The CLI also includes a `block-users` command that accepts CSV rows from standard input. It ignores all
columns except the first, which it expects to be a GitHub username. This is designed to make it convenient
to save the output of `list-pr-contributors`, manually remove accounts if needed, and then block the rest.

```
target/release/crabby -vvv -t $GH_TOKEN block-users < rms-contributors.csv
08:46:41 [WARN] 0hueliSJWpidorasi was already blocked
08:46:41 [WARN] 0kalekale was already blocked
...
```

If you've set the logging level to at least `WARN` (via the `-vvv` or `-vvvv` options), it will show you
a message for each user who is either newly blocked or was already blocked.

### Other tools

You can view all currently supported commands with `-h`:

```
crabby 0.1.0
Travis Brown <travisrobertbrown@gmail.com>

USAGE:
    crabby [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -v, --verbose    Logging verbosity
    -V, --version    Prints version information

OPTIONS:
    -t, --token <token>    A GitHub personal access token (not needed for all operations)

SUBCOMMANDS:
    block-users             Block a list of users provided in CSV format to stdin
    check-follow            Check whether one user follows another
    help                    Prints this message or the help of the given subcommand(s)
    list-blocks             List accounts the authenticated user blocks in CSV format to stdout
    list-followers          List the authenticated user's followers in CSV format to stdout
    list-following          List accounts the authenticated user follows in CSV format to stdout
    list-pr-contributors    List PR contributors for the given repository
```

## Caveats and future work

I wrote this thing yesterday afternoon. It's completely untested. It might not work. For your own safety
please don't use it with a personal access token with unneeded permissions (i.e. anything except `user`).

It's probably possible to include the account age in the contributor report even when unauthenticated—I just
wasn't able to find a way to get information about multiple users via a single request except through the
GraphQL endpoint, which is only available to authenticated users (and if you request each user individually,
you'll run into GitHub's rate limits for projects like the Stallman support letter).

## Related projects

* [highlight-rms-supporters] is a userscript that highlights signers of the Stallman support letter in the browser

## License

This project is licensed under the Mozilla Public License, version 2.0. See the LICENSE file for details.

[1375333996398325762]: https://twitter.com/travisbrown/status/1375333996398325762
[cancel-culture]: https://github.com/travisbrown/cancel-culture
[cargo]: https://doc.rust-lang.org/cargo/
[data-directory]: https://github.com/travisbrown/octocrabby/tree/main/data
[github]: https://github.com/
[github-token]: https://docs.github.com/en/github/authenticating-to-github/creating-a-personal-access-token
[highlight-rms-supporters]: https://github.com/sticks-stuff/highlight-RMS-supporters
[octocrab]: https://github.com/XAMPPRocky/octocrab
[rms-support-letter]: https://github.com/rms-support-letter/rms-support-letter.github.io
[rust]: https://www.rust-lang.org/
[rust-installation]: https://doc.rust-lang.org/book/ch01-01-installation.html