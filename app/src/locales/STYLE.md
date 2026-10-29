# PCTwin words: voice and word list

Every line on every screen follows this page. English (`en/main.ftl`) is the source; the other six
languages are machine-translated ahead of time from it (`app/l10n/termbase.json` holds the approved
term in each language, and `app/l10n/cards/` the per-language notes).

## Voice

Calm, plain and specific. PCTwin talks like a careful person doing the move by hand next to you:
it says what is happening, what happened, and what you do next. Nothing more.

- **Say the fact, not a feeling.** "The move is finished", not "Welcome to your next laptop".
- **Headings say what is happening or what you must decide.** "Choose the folders to move",
  "Accept the move from { $from }?". No full stop at the end of a heading.
- **Buttons are a verb, plus a noun when the verb alone is unclear.** 1 to 3 words, sentence case,
  no end punctuation: "Send files", "Choose a folder", "Resume the move". Never "OK", "Submit", "Yes".
  The common verbs are fixed: Continue, Resume, Pause, Stop, Try again, Check again, Cancel, Close, Done.
- **Results are numbers and states.** "12 of 14 copied and checked". Never cheer, never "all set".
  Never call something safe, done or ready unless PCTwin checked it.
- **Errors say what happened, then what to do.** No blame ("You said no"), no "Oops", no "Something
  went wrong" on its own. Use "please" only when the person is asked to wait.
- **PCTwin is "PCTwin", the person is "you".** Never "we", "us", "let’s".
- **No metaphors, idioms or slogans.** Not "on its way", "a fresh look", "where things got to",
  "give every file a home", "what matters", "bring it back", "settling you in", "in view".
- **Banned words:** magic, seamless, simply, just (as filler), journey, effortless, easy, "we’ve got
  you", exclamation marks, decorative emoji.
- **Technical detail goes behind More details.** The main line stays in everyday words.
- **Global.** No country-specific examples, money, holidays or services in the main flow.
- **Keep safety and privacy explanations whole.** Short is good; vague is not.

## Word list (use these, the same way, everywhere)

| Say | Meaning | Do not say |
|---|---|---|
| the move | a full move from one laptop to another | migration, transfer job, journey |
| old laptop | the laptop things move from (it is only read) | source, donor, previous device |
| new laptop | the laptop things move to | target, destination device, next laptop |
| this laptop / the other laptop | when the role is not the point | this device, peer |
| your things | files, apps and settings together | your stuff, your world, what matters |
| files / folders / apps / settings / app data | the kinds of things | content, assets, payload |
| item | one line in a list or report | entity, object |
| pair / paired | connect two laptops with the code and number | link up, sync, handshake |
| the code | the six-digit code on the old laptop | PIN, token, passcode |
| the number | the number you match on both laptops | security code, fingerprint |
| the plan | what PCTwin will do, before you approve it | proposal, manifest |
| approve / approval | you agree to the plan or a system step | say yes, give the go-ahead |
| accept | the new laptop agrees to what the old laptop chose | take, receive the offer |
| copied and checked | arrived and matched the original | verified, committed, landed |
| Copied, Reinstalled, Converted | finished result states | done, success |
| Needs action / needs you | the person must do a step | needs a look, attention required |
| Skipped by you / left out | the person chose not to move it | ignored, excluded |
| Failed / did not copy | it did not finish, with the reason | errored, broke |
| pause / paused, resume | stop for now, carry on from the same place | safe point, suspend |
| stop | end the rest of the move; what is copied stays | abort, kill |
| undo | put this laptop back as it was, keeping your later edits | rollback, revert |
| report | the item-by-item record of the move | log, receipt |
| move history | the list of past moves on this laptop | timeline |
| online only | a file stored in a cloud service and not on the laptop | in the cloud, cloud-only |
| your online storage | the service that holds online-only files | your cloud |
| drive | a disk or USB drive | volume, partition |
| free space | room left on a drive | capacity, quota |
| administrator | the account that can approve system changes | admin, root, elevation |
| Quick Send / send / receive | sending a few files to a laptop nearby | share, beam, drop |
| the sending laptop / the receiving laptop | the two sides of a Quick Send | sender device, recipient |
| Continue | go to the next step | Next, Proceed, Go on, Carry on (on buttons) |
| Try again / Check again | repeat the same step | Retry, Refresh |
| More details | the technical part, folded away | Advanced, Learn more |

## Checklist before a line ships

1. Would a person who has never used a computer setup tool follow it alone?
2. Does a heading say what is happening or what to decide?
3. Is every result a number or a state that PCTwin actually checked?
4. Does every error say what happened and what to do next?
5. Are placeholders (`{ $count }`), plural forms and markup exactly as before?
