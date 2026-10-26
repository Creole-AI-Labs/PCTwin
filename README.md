# PCTwin

**Your setup. Your next laptop.**

**Where we are:** see [UPDATE.md](UPDATE.md) for the latest progress, updated as work lands.

PCTwin moves everything from your old laptop to your new one, whatever the brand or system: Windows, Mac or Linux. Pair the two laptops with a short code, and PCTwin moves your files, reinstalls your apps fresh and carries your settings across, then gives you a clear report of anything you still need to do.

Moving your own laptops is free, with no limits.

## How a move works

**First, PCTwin learns what you have.** It looks at the laptops, the people who use them, their apps and their data. For each app it works out where a fresh copy will come from. For your data it works out what can be copied straight across, what needs the app's own export, what comes back when you sign in to a service, and what has to be set up again on the new laptop.

**Next, you see the plan.** Before anything changes, PCTwin shows you what will change, what needs you, how much it will download and how much space it needs, and anything it can't bring across yet. Nothing happens until you approve it.

**Then the work happens side by side.** An app can download while your files move. Some things wait for others: your private data is only brought in once the right app and any keys it needs are ready. Two jobs never work on the same thing at once.

**Every change is written down first.** PCTwin records what it is about to do and what really happened. If the connection drops or the app closes, it checks what really happened before trying again, and the work that finished stays finished.

**Finally, PCTwin checks you can really use your new laptop.** It brings together what was installed, moved, imported and set up, offers to fix only what failed, and tells you plainly what it checked. Something can be safe on the new laptop and still need you to sign in, add a key or install a matching app.

**Only then does PCTwin help you hand on the old laptop.** Wiping it is always a separate decision that you make yourself.

**One rule runs through all of it: each part of PCTwin can only do its own job.** The part that installs apps can't read your mail. The screens can't announce an install as finished. Approving a move never approves a wipe. No AI makes these decisions: every step follows fixed, tested rules.

We're building this now. See [UPDATE.md](UPDATE.md) for what is already done.

## Why the code is public

PCTwin needs deep access to your laptops. Publishing the transfer engine lets anyone check that it does what we say and nothing more: your files go straight from one laptop to the other over an encrypted link, the old laptop is never changed, and every change on the new one can be undone.

## Status

Early development. Nothing here is ready to move real data yet.

## Built with

- **Rust** for the transfer engine
- **Tauri 2** for the desktop app
- **React** with **TypeScript** for the app's screens

## Reporting problems

- **Bugs and ideas:** open an issue.
- **Security problems:** please don't open a public issue. See [SECURITY.md](SECURITY.md).

PCTwin is written by Creole AI Labs, and we don't accept code contributions. See [CONTRIBUTING.md](CONTRIBUTING.md) for why, and for how you can still help.

## Licence

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE). "PCTwin" and the PCTwin logo are trademarks of Creole AI Labs and are not covered by the code licence.
