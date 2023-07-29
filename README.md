<div align="center">

# glit

**glit** is a little osint tool to retrieve all mails of user related to a git repository, a git user or a git organization.

**README Sections:**  [Use](#use) — [Installation](#installation)

<img src="./img/demo.gif">
<br></br>
</div>


# Use

## Commands

```bash
Usage: glit [OPTIONS] [COMMAND]

Commands:
  repo  Extract emails from repository
  org   Extract emails from all repositories of a github organisation.
  user  Extract emails from all repositories of a user
  help  Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose        Add information on commit hash, username ...
  -o, --output <PATH>  export data to json
  -h, --help           Print help information
  -V, --version        Print version information
```

#### **Repository**

Fetch emails of all user/committer related to a repository.

```bash
glit repo -u https://github.com/rust-lang/git2-rs
```

#### **User**

Fetch emails of all user/committer from all repositories of a user.

```bash
glit user -u https://github.com/rust
```

#### **Organization**

Fetch emails of all user/committer from all repositories of an organization.

```bash
glit org -a -u https://github.com/netflix
```

## Other options

- -a , --all-branches : Search in all branches
- -o , --output : Write output as **JSON**

# Installation

### With cargo

```bash
cargo install glit-cli
```

### From Github Release

[Download a release](https://github.com/shadawck/glit/releases/lastest), extract and run.

```bash
tar -xvf glit-x86_64-unknown-linux-gnu-v0.2.0.tgz
mv glit /usr/local/bin/
```

## ROADMAP

- [ ] Change git2 for Gitoxide via the [git_repository](https://docs.rs/git-repository/latest/git_repository/) crate.
  - Use rustls-tls
