# Glit

Osint tool - Retrieve all mails of user related to a git repository, a git user or a git organization.

## Install

```bash
cargo install glit
```

## Use

### Repository

Fetch emails of all user/committer related to a repository.

```bash
glit repo -a -u https://github.com/rust-lang/git2-rs/
```

### User

Fetch emails of all user/committer from all repositories of a user.

```bash
glit user -a -u https://github.com/rust
```

### Organization

Fetch emails of all user/committer from all repositories of an organization.

```bash
glit org -a -u https://github.com/rust-lang
```