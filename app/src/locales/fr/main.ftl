# PCTwin screens, French (fr). Machine-translated ahead of time from ../en/main.ftl;
# a string missing here falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = Accueil PCTwin
language-label = Langue
motion-label = Animations
motion-system = Animations : comme le système
motion-reduced = Animations : réduites
motion-off = Animations : aucune

# The opening (Wordplay); its tagline is the home headline.
opening-label = PCTwin s’ouvre
opening-status = Ouverture de PCTwin.
opening-skip = Passer l’intro
opening-tagline = Envoyez des fichiers, ou transférez tout.

footer-free = Gratuit. Aucun compte nécessaire.
footer-help = Aide

## F01 Home

welcome-title-line1 = Envoyez des fichiers,
welcome-title-line2 = ou transférez tout.
welcome-body = Envoyez quelques fichiers à un ordinateur portable à proximité, ou emportez vos fichiers, vos applis et vos réglages sur votre nouvel ordinateur portable.
welcome-start = Commencer un transfert complet

## F01 Home: the cards, My history, a move to resume, files waiting

home-quick-send-body = Envoyer ou recevoir des fichiers
home-move-body = Transférer votre configuration
home-restore-body = Restaurer une configuration
home-rescue-body = Copier les fichiers d’un portable qui ne démarre plus
home-history = Mon historique
home-history-body = Vos transferts et leurs rapports
name-title = Donnez un nom à cet ordinateur
name-body = Les autres ordinateurs à proximité le trouveront sous ce nom. Vous pourrez le changer plus tard, au moment de l’associer.
name-use = Utiliser ce nom
home-update-title = Une nouvelle version est disponible
home-update-action = Mettre à jour
home-updating = Mise à jour en cours. PCTwin va se fermer et se rouvrir tout seul.
home-update-failed = La mise à jour n’a pas pu être installée. PCTwin fonctionne comme avant ; elle vous sera proposée à nouveau la prochaine fois.
home-update-busy = Terminez d’abord votre transfert. Ensuite, mettez PCTwin à jour.
home-resume-body = Votre progression est enregistrée.
home-resume-action = Reprendre
    .aria-label = Reprendre votre transfert
home-waiting =
    { NUMBER($count) ->
        [one] { $count } fichier en attente pour { $to }
        [many] { $count } de fichiers en attente pour { $to }
       *[other] { $count } fichiers en attente pour { $to }
    }

## Header: this laptop, the language and motion control, Help

header-this-laptop = Votre ordinateur

## F06 Old or new laptop

role-back = Retour
role-eyebrow = Commencer un transfert complet
role-title = Quel ordinateur portable est-ce ?
role-body = Ouvrez PCTwin sur les deux ordinateurs portables. Puis indiquez lequel est celui-ci.
role-old-title = C’est mon ancien ordinateur portable
role-old-body = Envoyer mes fichiers, applis et réglages.
role-new-title = C’est mon nouvel ordinateur portable
role-new-body = Recevoir ma configuration d’un autre ordinateur portable.
role-note = Rien n’est transféré avant que vous ayez vérifié et approuvé le plan. Votre ancien ordinateur portable reste inchangé pendant le transfert.

## Frame parts (footer note, Back, steps)

footer-note-move = Transferts personnels gratuits
footer-note-send = Envoi gratuit à proximité
frame-back = Retour
# Read out by screen readers only; the step shapes carry the meaning on screen.
frame-step-done = Terminé
frame-step-current = En cours
frame-step-todo = À venir
frame-step-position = Étape { $number } sur { $total }

## F01 Unfinished move

resume-open = Reprendre votre transfert

## C01 How will you connect?

connect-eyebrow = Votre transfert
connect-title = Quelle connexion allez-vous utiliser ?
connect-body = Quel que soit votre choix, les étapes sont les mêmes.
connect-wifi-title = Wi-Fi
connect-wifi-body = Les deux ordinateurs portables sur le même Wi-Fi.
connect-wifi-status = Disponible
connect-later = Un câble réseau, le câble passerelle PCTwin et le transfert par un disque externe sont prévus pour une version ultérieure.
connect-usb-note = Un câble USB ordinaire ne permet pas de connecter deux ordinateurs portables pour un transfert.
connect-use-wifi = Utiliser le Wi-Fi
# Added: sizes and speeds as measured, shown the same way on every connection screen.
connect-unit-mbps = { NUMBER($value, maximumFractionDigits: 0) } MB/s
connect-unit-gb = { NUMBER($value, maximumFractionDigits: 1) } GB

## C02 Put both laptops together

connect-prep-title = Rapprochez les deux ordinateurs portables
connect-prep-body = Connectez-les au même Wi-Fi, puis ouvrez PCTwin sur chacun.
connect-prep-step-1-title = Rejoindre le même Wi-Fi
connect-prep-step-1-body = La copie des fichiers n’a pas besoin d’internet.
# $network is the Wi-Fi name this laptop is on.
connect-prep-step-1-network = Réseau : { $network }.
connect-prep-step-2-title = Autoriser la recherche à proximité
connect-prep-step-2-body = Si Windows ou macOS demande une autorisation, choisissez Autoriser.
connect-prep-step-3-title = Choisir un rôle sur chaque ordinateur portable
connect-prep-step-3-body = L’un envoie. L’autre reçoit.
connect-prep-note = Un Wi-Fi invité ou public peut empêcher les ordinateurs portables de se voir.
connect-prep-continue = Passer à l’association

## C06 Meet your other laptop

peer-eyebrow = Votre transfert
peer-title = Vérifiez les deux portables
peer-body = Les deux ordinateurs portables sont connectés et prêts.
peer-sending = Envoi
peer-receiving = Réception
peer-old = Ancien ordinateur portable
peer-new = Nouvel ordinateur portable
peer-version-label = PCTwin
# $version is the PCTwin version on that laptop, for example "1.0.3".
peer-version = { $version }
peer-status-update = Mise à jour nécessaire
peer-status-unsupported = En attente
peer-ready-note = PCTwin affiche vos fichiers seulement une fois les deux ordinateurs portables associés. Si les versions diffèrent, mettez PCTwin à jour sur les deux.
peer-version-differs = Ces ordinateurs portables n’ont pas la même version de PCTwin. Mettez PCTwin à jour sur les deux pour continuer.
peer-version-old-body = Version différente : { $version }.
peer-version-new-body = La même version est nécessaire sur les deux ordinateurs portables.
peer-update-title = Mettre PCTwin à jour
peer-update-body = Installez la même version sur les deux ordinateurs portables.
# $from and $to are system names, for example "Windows 11" and "macOS". The mockup sentence does not use them.
peer-direction-unsupported = Cette version de PCTwin ne peut pas encore faire de transfert entre ces deux systèmes.
peer-continue = Vérifier la connexion

## C07 Let’s check the connection

check-eyebrow = Votre transfert
check-title = Vérifiez la connexion
check-body = PCTwin vérifie la vitesse, l’espace et l’alimentation pour préparer le transfert et estimer sa durée.
check-measuring = Mesure de la connexion…
# The value next to it is already formatted, for example "62 MB/s".
check-speed-label = vitesse de connexion
check-space-label = libre sur le nouvel ordinateur
check-link-weak = Connexion faible, le transfert risque d’être lent. Rapprocher les ordinateurs portables du routeur Wi-Fi peut aider.
check-power-title = Alimentation
check-power-both = Les deux ordinateurs sont branchés.
check-power-battery = { $label } fonctionne sur batterie. Branchez-le pour qu’un long transfert ne soit pas interrompu.
check-space-title = Stockage
check-space-body = { $free } d’espace libre sur le nouvel ordinateur portable. Avant tout transfert, vous verrez si vos choix tiennent.
check-internet-title = Internet
check-internet-on = Connecté. La copie des fichiers n’a pas besoin d’internet.
check-internet-off = Non connecté. Ce n’est pas grave : la copie des fichiers n’a pas besoin d’internet.
check-status-ready = Prêt
check-status-plug-in = À brancher
check-status-connected = Disponible
check-status-not-needed = Pas nécessaire
# Added: what is said when something could not be read, so it is never shown as fine.
check-status-unknown = Inconnu
check-speed-unknown = PCTwin n’a pas pu mesurer la connexion cette fois. Les fichiers sont quand même transférés ; seule l’estimation du temps manque.
check-space-unknown = PCTwin n’a pas pu lire l’espace libre sur le nouvel ordinateur portable.
check-power-unknown = PCTwin ne sait pas si { $label } est branché.
check-internet-unknown = PCTwin ne sait pas si cet ordinateur portable est en ligne. Ce n’est pas grave : la copie des fichiers n’a pas besoin d’internet.
check-again = Vérifier à nouveau
check-estimate-note = La durée estimée n’est pas une promesse.
check-continue = Choisir quoi transférer
check-continue-new = Continuer
# Added 23 October 2026: the short checklist both laptops show before the move.
check-before-title = Avant le début du transfert
check-before-lids-title = Gardez les deux écrans ouverts
check-before-lids-body = Un écran fermé peut mettre un ordinateur portable en veille, ce qui met le transfert en pause.
check-before-near-title = Restez près du routeur Wi-Fi
check-before-near-body = Gardez les deux ordinateurs portables près de lui pendant le transfert.

## C09 Waiting for your other laptop

reconnect-title = En attente de votre autre ordinateur portable
reconnect-body = Tout ce qui a été copié et vérifié jusqu’ici est gardé.
# Added: the title of the saved-progress card (the mockup shows Move paused by connection).
reconnect-card-title = Transfert en pause, connexion coupée
reconnect-progress =
    .aria-label = Transfert en pause, connexion coupée
# $done is an already formatted size, for example "81.6 GB". $total is passed but not shown.
reconnect-saved = { $done } copiés et vérifiés jusqu’ici. Le reste va suivre.
# $ago is already formatted, for example "45 seconds ago".
reconnect-last-seen = Vu { $ago }.
reconnect-status-offline = Hors ligne
reconnect-status-trying = Reconnexion
reconnect-status-pending = En attente
reconnect-trying = Reconnexion automatique en cours… Gardez les deux ordinateurs portables allumés, sans mise en veille et sur le même Wi-Fi.
reconnect-expired = Trop de temps s’est écoulé pour une reconnexion automatique. Associez-les à nouveau, et PCTwin vérifiera ce qui a déjà été copié.
reconnect-unavailable = { $label } est injoignable. Vérifiez qu’il est allumé, sans mise en veille, avec PCTwin ouvert et sur le même Wi-Fi.
reconnect-try-now = Réessayer la connexion
reconnect-pair-again = Associer à nouveau
reconnect-nothing-new = Rien de nouveau n’est arrivé depuis un moment. PCTwin continue d’essayer tout seul, ou vous pouvez arrêter le transfert. Tout ce qui a été copié et vérifié jusqu’ici est conservé.
reconnect-details = Une reconnexion sûre
reconnect-details-body = Seuls ces deux ordinateurs portables peuvent se reconnecter, et ils utilisent à chaque fois une nouvelle connexion privée.

## C10 The connection has ended (dialog)

ended-title = La connexion est terminée
ended-body = Votre transfert est enregistré. L’ancien code ne fonctionne plus.
ended-kept-title = Gardé
ended-kept-body = Le relevé de votre transfert et tout ce qui a été copié et vérifié.
ended-not-kept-title = Non gardé
ended-not-kept-body = L’autorisation de continuer sans nouvelle association.
ended-pair-again = Associer à nouveau
ended-pair-again-body = Connectez-les à nouveau, et PCTwin vérifiera ce qui a déjà été copié.
ended-see-moved = Ouvrir le transfert enregistré
ended-see-moved-body = Voir ce qui a été copié et vérifié jusqu’ici.
ended-home = Retour à l’accueil

## Pairing, shared

pair-back = Retour
pair-try-again = Réessayer
pair-more-details = Plus de détails
pair-heads-up-title = Votre ordinateur peut vous poser une question
pair-heads-up-local-network = Votre Mac va demander si PCTwin peut trouver des appareils sur votre réseau local. Choisissez Autoriser, pour que vos deux ordinateurs portables puissent se trouver.
pair-heads-up-firewall = Windows peut demander si PCTwin peut accepter des connexions. Choisissez Autoriser, pour que votre nouvel ordinateur portable puisse joindre celui-ci.
pair-heads-up-continue = Continuer

## C03 Old laptop: show the code

pair-old-eyebrow = Ancien ordinateur portable
pair-old-title = Connectez vos ordinateurs portables
pair-old-body = Sur votre nouvel ordinateur portable, saisissez le code ci-dessous.
pair-old-next = Ensuite, vous choisirez le numéro affiché sur les deux ordinateurs portables.
pair-old-network = Gardez les deux ordinateurs portables sur le même Wi-Fi.
pair-old-waiting = En attente de votre nouvel ordinateur portable…
pair-old-starting = Préparation d’un code…
pair-old-label-title = Nom de cet ordinateur portable
pair-old-label-hint = Votre nouvel ordinateur portable trouvera celui-ci sous ce nom.
pair-old-shuffle = Choisir un autre nom
pair-old-rename = Saisir un nom
pair-old-rename-label = Nom de cet ordinateur portable
pair-old-rename-note = Toute personne sur ce Wi-Fi peut voir ce nom.
pair-old-rename-save = Utiliser ce nom
pair-old-rename-cancel = Annuler
pair-old-code-meta =
    { $seconds ->
        [one] Change chaque minute. Nouveau code dans { $seconds } seconde.
        [many] Change chaque minute. Nouveau code dans { $seconds } secondes.
       *[other] Change chaque minute. Nouveau code dans { $seconds } secondes.
    }
pair-old-steps-title = Sur le nouvel ordinateur portable
pair-old-step-1 = Ouvrez PCTwin et choisissez Mon nouvel ordinateur portable.
pair-old-step-2 = Si une liste apparaît, choisissez le nom de cet ordinateur portable.
pair-old-step-3 = Saisissez le code à six chiffres affiché ici.
pair-old-wait-title = Attendez un nouveau code
pair-old-wait-body =
    { $seconds ->
        [one] Cet essai n’a pas fonctionné. Un nouveau code apparaît dans { $seconds } seconde.
        [many] Cet essai n’a pas fonctionné. Un nouveau code apparaît dans { $seconds } secondes.
       *[other] Cet essai n’a pas fonctionné. Un nouveau code apparaît dans { $seconds } secondes.
    }
pair-old-locked-title = Quelqu’un sur ce réseau a essayé de mauvais codes
pair-old-locked-body = Après cinq mauvais codes, l’association s’est arrêtée pour protéger cet ordinateur portable. Rien n’a été partagé. Si c’était vous, choisissez Recommencer pour obtenir un nouveau code.
pair-old-locked-hotspot = Si vous ne savez pas qui c’était, connectez plutôt les deux ordinateurs portables au partage de connexion de votre téléphone, puis choisissez Recommencer.
pair-old-start-again = Recommencer
pair-old-timed-out = Aucun numéro n’a été choisi à temps, voici donc un nouveau code.
pair-old-declined = Les numéros ne correspondaient pas, rien n’a donc été partagé. Voici un nouveau code.
pair-old-left = Le nouvel ordinateur portable a arrêté l’association avant le choix d’un numéro, rien n’a donc été partagé. Voici un nouveau code.
pair-network-title = Choisissez votre Wi-Fi
pair-network-hint = Choisissez le réseau de votre nouvel ordinateur portable.

## C05 Old laptop: match the number

pair-choose-eyebrow = Confirmer la connexion
pair-choose-title = Choisissez le numéro correspondant
pair-choose-body = Regardez votre nouvel ordinateur portable. Choisissez le numéro qu’il affiche.
pair-choose-number = Numéro { $number }
pair-choose-confirm = Confirmer ces ordinateurs portables
pair-choose-renewed = De nouveaux numéros sont affichés. Choisissez celui de votre nouvel ordinateur portable.
pair-choose-none = Aucun ne correspond
pair-choose-note = Si le nouvel ordinateur portable n’affiche aucun numéro, ou si aucun ne correspond, choisissez Aucun ne correspond.
pair-choose-safe = Rien n’a encore été partagé.
pair-choose-checking = Vérification…

## C04 New laptop: find the old laptop and enter the code

pair-new-eyebrow = Nouvel ordinateur portable
pair-new-searching-title = Recherche de votre ancien ordinateur portable…
pair-new-searching-body = Gardez PCTwin ouvert sur votre ancien ordinateur portable, sur le même Wi-Fi que celui-ci.
pair-new-search-failed-title = Impossible de chercher votre ancien portable
pair-new-list-title = Lequel est votre ancien ordinateur portable ?
pair-new-list-body = Choisissez le nom affiché sur l’écran de votre ancien ordinateur portable.
pair-new-same-label = Un autre ordinateur portable à proximité porte ce nom. Vérifiez l’écran de l’ancien ordinateur.
pair-new-needs-update = Cet ordinateur a une version plus ancienne de PCTwin. Mettez PCTwin à jour dessus, puis recherchez à nouveau.
pair-new-none-title = Ancien ordinateur introuvable
pair-new-none-body = Vérifiez que PCTwin est ouvert sur votre ancien ordinateur portable et affiche un code, et que les deux ordinateurs sont sur le même Wi-Fi. Puis relancez la recherche.
pair-new-search-again = Rechercher à nouveau
pair-old-same-role = Un autre portable à proximité est aussi réglé comme l’ancien portable. L’un des deux doit être le nouveau portable.
pair-old-switch = Ce portable est le nouveau
pair-new-none-same-role = L’autre portable a-t-il aussi choisi Nouveau portable ? Choisissez alors Ancien portable sur celui-là, ou changez celui-ci.
pair-new-switch = Ce portable est l’ancien
pair-new-code-title = Saisissez le code
pair-new-code-body = Saisissez le code affiché sur { $label }.
pair-new-code-label = Code à six chiffres
pair-new-code-hint = Lisez-le sur l’écran de votre ancien ordinateur portable. Il change chaque minute.
pair-new-connect = Connecter
pair-new-connecting = Connexion…
pair-new-other-laptop = Choisir un autre ordinateur portable
pair-new-number-eyebrow = Confirmer la connexion
pair-new-number-title = Choisissez ce numéro sur votre ancien ordinateur portable
pair-new-number-tile = Votre numéro
pair-new-number-label = Votre numéro est { $number }
pair-new-number-body = Votre ancien ordinateur portable affiche trois numéros. Choisissez celui-ci sur l’ancien.
pair-new-number-waiting = En attente de votre ancien ordinateur portable…

## Paired (both laptops)

pair-done-eyebrow = Associés
pair-done-title = Vos ordinateurs portables sont associés
pair-done-body = Vos deux ordinateurs portables sont connectés. L’ancien cherche maintenant vos fichiers, puis vous approuvez sur le nouveau ce qui est déplacé. Rien ne bouge avant votre accord.
pair-done-continue = Continuer

## Pairing problems (each says what happened, then what to do)

pairing-code-invalid = Ce code doit comporter six chiffres. Vérifiez l’écran de l’ancien ordinateur portable et saisissez-le à nouveau.
pairing-code-wrong = Ce code ne correspond pas. Vérifiez l’écran de l’ancien ordinateur portable et réessayez.
pairing-code-expired = Ce code a changé. Saisissez le nouveau code affiché sur l’écran de l’ancien ordinateur portable.
pairing-busy = L’ancien ordinateur portable est déjà en cours d’association avec un autre. Attendez un instant, puis réessayez.
pairing-wait = L’ancien ordinateur portable prépare un nouveau code. Attendez-le, puis saisissez le nouveau code.
pairing-locked = L’association s’est arrêtée après trop d’essais erronés. Sur l’ancien ordinateur portable, choisissez Recommencer.
pairing-unreachable = Cet ordinateur portable n’a pas pu joindre l’ancien. Vérifiez que les deux sont sur le même Wi-Fi, puis réessayez.
pairing-wrong-number = Ce numéro ne correspond pas à celui du nouvel ordinateur portable. Rien n’a été partagé. Saisissez le nouveau code sur le nouvel ordinateur pour réessayer.
pairing-no-match = Les numéros ne correspondaient pas sur l’ancien ordinateur portable. Rien n’a été partagé. Saisissez le nouveau code affiché sur l’écran de l’ancien ordinateur.
pairing-timed-out = Aucun numéro n’a été choisi à temps. Réessayez avec le nouveau code de l’ancien ordinateur portable.
pairing-connection-lost = La connexion entre les ordinateurs portables a été coupée. Rien n’a été partagé. Réessayez.
pairing-connection-lost-new = La connexion avec l’ancien ordinateur portable a été coupée. Rien n’a été partagé.
pairing-different-version = Les deux ordinateurs portables n’ont pas la même version de PCTwin. Mettez PCTwin à jour sur les deux, puis réessayez.
pairing-name-invalid = Ce nom ne peut pas être utilisé. Essayez un nom plus court, avec des lettres et des chiffres.
pairing-no-network = Cet ordinateur portable n’est sur aucun réseau Wi-Fi ou filaire. Connectez-le, puis réessayez.
pairing-choose-network = Cet ordinateur portable est sur plusieurs réseaux. Choisissez celui de votre nouvel ordinateur :
pairing-local-network-refused = Votre Mac empêche peut-être PCTwin de trouver des ordinateurs portables sur ce réseau. Si votre Mac vient de vous le demander, choisissez Autoriser, puis réessayez. Sinon, ouvrez Réglages Système, choisissez Confidentialité et sécurité, puis Réseau local, activez PCTwin et réessayez.
pairing-local-network-refused-details = Un VPN, un pare-feu ou une app de filtrage peut aussi empêcher les ordinateurs portables de se trouver. S’il y en a un d’actif, désactivez-le le temps de l’association, puis réessayez.
pairing-failed = L’association n’a pas fonctionné cette fois. Rien n’a été partagé. Réessayez.
pairing-failed-new = L’association n’a pas fonctionné cette fois. Rien n’a été partagé. Saisissez le nouveau code affiché sur l’ancien portable.

## M06 Match people to their profiles (Who’s moving?)

people-eyebrow = Votre transfert
people-title = Choisissez le compte de chaque personne
people-body = Choisissez où vont les fichiers de chaque personne sur le nouvel ordinateur portable.
people-col-old = Ancien ordinateur portable
people-col-new = Nouvel ordinateur portable
people-col-status = État
people-you = Vous
# $name is the person's name on the old laptop. $account is an account name on the new laptop.
people-select =
    .aria-label = Compte sur le nouvel ordinateur portable pour { $name }
people-option-existing = { $account } / nouvel ordinateur portable
people-option-new = Créer un nouveau compte
people-option-leave-out = Ne pas inclure
people-option-choose = Choisir…
people-new-account-hint = Le nouvel ordinateur portable demande une autorisation avant de créer un compte.
people-suggested = Suggéré car les noms correspondent. Vérifiez qu’il s’agit de la même personne.
# $account is an account name on the new laptop.
people-suggested-use = Utiliser { $account }
people-status-ready = Confirmé
people-status-waiting = Doit donner son accord
people-status-left-out = Non inclus
people-status-choose = Votre choix est nécessaire
people-status-changed = Modifié par vous
people-waiting-body = { $name } doit se connecter sur l’ancien ordinateur portable et donner son accord pour que ses fichiers soient transférés. D’ici là, ses fichiers restent intacts.
people-just-me = Seul votre propre compte a été trouvé sur l’ancien ordinateur portable.
people-changed-note = Vous avez changé l’endroit où vont les fichiers d’une personne. Vérifiez-le avant de continuer.
people-shared-title = Éléments partagés
people-shared-body = Les dossiers partagés et les documents publics qui appartiennent à tous sur l’ancien ordinateur portable.
people-shared-select =
    .aria-label = Destination des éléments partagés
people-shared-option-shared = Dossiers partagés du nouvel ordinateur portable
people-shared-option-person = Avec les fichiers de { $name }
people-shared-option-leave-out = Ne pas inclure
people-note = PCTwin ne change jamais le mot de passe de quiconque et ne contourne jamais la connexion d’une autre personne.
people-list-label = Personnes sur l’ancien ordinateur portable
people-todo =
    { $count ->
        [0] Chaque personne à transférer a une destination.
        [one] { $count } personne attend encore un choix.
        [many] { $count } de personnes attendent encore un choix.
       *[other] { $count } personnes attendent encore un choix.
    }
people-waiting-count =
    { $count ->
        [one] { $count } personne doit encore donner son accord. Vous pouvez continuer ; ses fichiers restent intacts jusqu’à son accord.
        [many] { $count } de personnes doivent encore donner leur accord. Vous pouvez continuer ; leurs fichiers restent intacts jusqu’à leur accord.
       *[other] { $count } personnes doivent encore donner leur accord. Vous pouvez continuer ; leurs fichiers restent intacts jusqu’à leur accord.
    }
people-continue = Continuer

## M26 Before you leave this laptop (things to do on the old laptop)

before-eyebrow = Votre transfert
before-title = Avant de quitter cet ordinateur portable
before-body = Vérifiez ces points tant que vous avez encore votre ancien ordinateur portable.
before-licences-title = Licences des applis
before-licences-body = Pour certaines applis, vous devez transférer la licence vous-même.
before-passwords-title = Récupération des comptes
before-passwords-body = Assurez-vous de pouvoir toujours accéder à vos comptes et à vos disques verrouillés.
before-codes-title = Gardez vos codes de connexion à portée de main
before-codes-body = Si un compte vous envoie un code ou utilise une appli pour confirmer votre identité, gardez ce téléphone ou cette appli à proximité.
before-online-title = Travail non enregistré
before-online-body = Enregistrez vos brouillons et vérifiez que les fichiers stockés en ligne sont à jour.
before-done =
    .aria-label = C’est fait : { $item }
before-note = Ne saisissez ici ni mot de passe ni clé de récupération. Terminer cette liste ne veut pas dire que l’ancien ordinateur portable peut être effacé sans risque.
before-empty = Rien à faire sur votre ancien ordinateur portable avant le transfert.
before-continue = Revenir au plan du transfert
before-keys-title = Clés de licence trouvées sur cet ordinateur portable
before-keys-body = Ces applis gardent leur clé de licence sur cet ordinateur portable. Notez chaque clé ou conservez-la en lieu sûr avant de quitter cet ordinateur portable. PCTwin ne met aucune clé dans le transfert ni dans son rapport.
before-keys-show = Afficher la clé
before-keys-show-label = Afficher la clé de { $app }
before-keys-hide = Masquer la clé
before-keys-gone = PCTwin n’a pas pu relire cette clé. Ouvrez { $app } pour la trouver.

## S11 Get help with this task (a short problem code)

help-eyebrow = Aide et confidentialité
help-title = Obtenir de l’aide pour cette tâche
help-body = Partagez un code court qui montre à quelqu’un ce qui n’a pas marché. Vérifiez d’abord ce qu’il contient.
help-making = Création d’un code…
help-code-title = Code du problème
# $code is the code itself, for example "PCT-7F3K-21". Keep it unchanged.
help-code =
    .aria-label = Code du problème { $code }
help-contains-title = Ce qu’il contient
help-contains-problem = Le type de problème et l’avancement de la tâche.
help-contains-step = L’étape du transfert où il s’est produit.
help-contains-versions = La version de PCTwin et le système de ce portable, comme Windows ou macOS.
help-never-title = Ce qu’il ne contient jamais
help-chip-included = Inclus
help-chip-excluded = Jamais inclus
help-never-names = Votre nom ou celui de quelqu’un d’autre.
help-never-files = Le contenu de vos fichiers ou vos mots de passe.
help-never-network = Les noms de fichiers ou de dossiers, l’endroit où sont vos fichiers, ou les noms de Wi-Fi.
help-note = Ce code ne donne à personne l’accès à vos ordinateurs portables.
help-copy = Copier le code
help-copied = Code copié.
help-share = Partager
help-share-hint = Envoyez-le par WhatsApp ou une autre appli.
help-share-text = Mon code de problème PCTwin est { $code }.
help-failed = Impossible de créer un code de problème. Réessayez. Si cela ne marche toujours pas, décrivez plutôt le problème avec vos propres mots.
help-try-again = Réessayer
help-copy-failed = Le code n’a pas pu être copié. Sélectionnez-le ci-dessus et copiez-le vous-même.
help-home-eyebrow = Aide
help-home-title = Problèmes fréquents et que faire
help-home-body = De courts guides pour ce qui bloque le plus souvent. Ils fonctionnent sans internet.
help-home-more = Plus d’aide sur getpctwin.com
help-guide-find-title = Les portables ne se trouvent pas
help-guide-find-body = Connectez les deux portables au même Wi-Fi, pas à un réseau invité. Coupez tout VPN pendant l’association. Si cela ne marche toujours pas, activez le partage de connexion du téléphone et connectez-y les deux portables.
help-guide-mac-network-title = Votre Mac pose une question sur le réseau local
help-guide-mac-network-body = Choisissez Autoriser pour que PCTwin trouve votre autre portable. Si vous avez choisi Ne pas autoriser, ouvrez Réglages Système, puis Confidentialité et sécurité, puis Réseau local, et activez PCTwin.
help-guide-firewall-title = Windows pose une question sur le pare-feu
help-guide-firewall-body = Autorisez PCTwin sur les réseaux privés, comme le Wi-Fi de la maison. PCTwin ne parle qu’à votre autre portable, sur votre propre réseau.
help-guide-stopped-title = Un transfert s’est arrêté en cours de route
help-guide-stopped-body = Rien de ce qui a été copié et vérifié n’est perdu, et l’ancien portable n’a pas changé. Associez à nouveau les portables : les fichiers déjà sur le nouveau sont reconnus et ne sont pas copiés deux fois.
help-guide-others-title = Déplacer les affaires d’autres personnes
help-guide-others-body = Cochez-les dans Qui déménage ? sur l’ancien portable. Un administrateur donne son accord une fois sur chaque portable. Vous ne voyez jamais leurs fichiers. Le mot de passe temporaire d’un nouveau compte s’affiche une fois dans une fenêtre du portable ; la personne choisit le sien à sa première connexion.
help-guide-code-title = Demander de l’aide à quelqu’un
help-guide-code-body = Sur toute page de problème, choisissez Obtenir de l’aide. PCTwin crée un code court qui dit ce qui s’est passé, sans nom, fichier ni mot de passe, à envoyer par WhatsApp ou toute autre app.

## Label words (the name each laptop shows while pairing)

label-colour-red = rouge
label-colour-orange = orange
label-colour-yellow = jaune
label-colour-green =
    { $gender ->
        [feminine] verte
       *[masculine] vert
    }
label-colour-blue =
    { $gender ->
        [feminine] bleue
       *[masculine] bleu
    }
label-colour-purple =
    { $gender ->
        [feminine] violette
       *[masculine] violet
    }
label-colour-pink = rose
label-colour-brown = marron
label-colour-black =
    { $gender ->
        [feminine] noire
       *[masculine] noir
    }
label-colour-white =
    { $gender ->
        [feminine] blanche
       *[masculine] blanc
    }
label-colour-grey =
    { $gender ->
        [feminine] grise
       *[masculine] gris
    }
label-colour-gold =
    { $gender ->
        [feminine] dorée
       *[masculine] doré
    }

label-animal-ant = Fourmi
label-animal-badger = Blaireau
label-animal-bat = Pipistrelle
label-animal-bear = Ours
label-animal-bee = Abeille
label-animal-beetle = Scarabée
label-animal-bison = Bison
label-animal-butterfly = Papillon
label-animal-camel = Chameau
label-animal-cat = Chat
label-animal-cheetah = Guépard
label-animal-chicken = Poule
label-animal-crab = Crabe
label-animal-crane = Grue
label-animal-crocodile = Crocodile
label-animal-crow = Corbeau
label-animal-deer = Cerf
label-animal-dolphin = Dauphin
label-animal-donkey = Âne
label-animal-dove = Colombe
label-animal-dragonfly = Libellule
label-animal-duck = Canard
label-animal-eagle = Aigle
label-animal-elephant = Éléphant
label-animal-falcon = Faucon
label-animal-flamingo = Flamant
label-animal-fox = Renard
label-animal-frog = Grenouille
label-animal-gazelle = Gazelle
label-animal-swordfish = Espadon
label-animal-giraffe = Girafe
label-animal-goat = Chèvre
label-animal-goose = Oie
label-animal-gorilla = Gorille
label-animal-hamster = Hamster
label-animal-hawk = Buse
label-animal-hedgehog = Hérisson
label-animal-armadillo = Tatou
label-animal-hippo = Hippopotame
label-animal-horse = Cheval
label-animal-hummingbird = Colibri
label-animal-jaguar = Jaguar
label-animal-jellyfish = Méduse
label-animal-kangaroo = Kangourou
label-animal-kingfisher = Martin-pêcheur
label-animal-koala = Koala
label-animal-ladybird = Coccinelle
label-animal-lemur = Lémurien
label-animal-leopard = Léopard
label-animal-lion = Lion
label-animal-lizard = Lézard
label-animal-llama = Lama
label-animal-lobster = Homard
label-animal-lynx = Lynx
label-animal-macaw = Ara
label-animal-meerkat = Suricate
label-animal-mole = Taupe
label-animal-moose = Élan
label-animal-pufferfish = Poisson-globe
label-animal-mouse = Souris
label-animal-octopus = Pieuvre
label-animal-ostrich = Autruche
label-animal-otter = Loutre
label-animal-owl = Chouette
label-animal-panda = Panda
label-animal-kiwi = Kiwi
label-animal-parrot = Perroquet
label-animal-peacock = Paon
label-animal-pelican = Pélican
label-animal-penguin = Manchot
label-animal-puffin = Macareux
label-animal-rabbit = Lapin
label-animal-raccoon = Raton laveur
label-animal-rhino = Rhinocéros
label-animal-robin = Rouge-gorge
label-animal-salmon = Saumon
label-animal-seahorse = Hippocampe
label-animal-seal = Phoque
label-animal-shark = Requin
label-animal-sheep = Mouton
label-animal-sloth = Paresseux
label-animal-snail = Escargot
label-animal-sparrow = Moineau
label-animal-squirrel = Écureuil
label-animal-starfish = Étoile de mer
label-animal-stork = Cigogne
label-animal-swan = Cygne
label-animal-tiger = Tigre
label-animal-snake = Serpent
label-animal-toucan = Toucan
label-animal-turtle = Tortue
label-animal-walrus = Morse
label-animal-whale = Baleine
label-animal-wolf = Loup
label-animal-woodpecker = Pivert
label-animal-yak = Yak
label-animal-zebra = Zèbre
label-animal-dog = Chien
label-animal-cow = Vache
label-animal-beaver = Castor

# Grammatical gender of each animal noun above (not translated; read by labelText for colour agreement).
label-gender-ant = feminine
label-gender-badger = masculine
label-gender-bat = feminine
label-gender-bear = masculine
label-gender-bee = feminine
label-gender-beetle = masculine
label-gender-bison = masculine
label-gender-butterfly = masculine
label-gender-camel = masculine
label-gender-cat = masculine
label-gender-cheetah = masculine
label-gender-chicken = feminine
label-gender-crab = masculine
label-gender-crane = feminine
label-gender-crocodile = masculine
label-gender-crow = masculine
label-gender-deer = masculine
label-gender-dolphin = masculine
label-gender-donkey = masculine
label-gender-dove = feminine
label-gender-dragonfly = feminine
label-gender-duck = masculine
label-gender-eagle = masculine
label-gender-elephant = masculine
label-gender-falcon = masculine
label-gender-flamingo = masculine
label-gender-fox = masculine
label-gender-frog = feminine
label-gender-gazelle = feminine
label-gender-swordfish = masculine
label-gender-giraffe = feminine
label-gender-goat = feminine
label-gender-goose = feminine
label-gender-gorilla = masculine
label-gender-hamster = masculine
label-gender-hawk = feminine
label-gender-hedgehog = masculine
label-gender-armadillo = masculine
label-gender-hippo = masculine
label-gender-horse = masculine
label-gender-hummingbird = masculine
label-gender-jaguar = masculine
label-gender-jellyfish = feminine
label-gender-kangaroo = masculine
label-gender-kingfisher = masculine
label-gender-koala = masculine
label-gender-ladybird = feminine
label-gender-lemur = masculine
label-gender-leopard = masculine
label-gender-lion = masculine
label-gender-lizard = masculine
label-gender-llama = masculine
label-gender-lobster = masculine
label-gender-lynx = masculine
label-gender-macaw = masculine
label-gender-meerkat = masculine
label-gender-mole = feminine
label-gender-moose = masculine
label-gender-pufferfish = masculine
label-gender-mouse = feminine
label-gender-octopus = feminine
label-gender-ostrich = feminine
label-gender-otter = feminine
label-gender-owl = feminine
label-gender-panda = masculine
label-gender-kiwi = masculine
label-gender-parrot = masculine
label-gender-peacock = masculine
label-gender-pelican = masculine
label-gender-penguin = masculine
label-gender-puffin = masculine
label-gender-rabbit = masculine
label-gender-raccoon = masculine
label-gender-rhino = masculine
label-gender-robin = masculine
label-gender-salmon = masculine
label-gender-seahorse = masculine
label-gender-seal = masculine
label-gender-shark = masculine
label-gender-sheep = masculine
label-gender-sloth = masculine
label-gender-snail = masculine
label-gender-sparrow = masculine
label-gender-squirrel = masculine
label-gender-starfish = feminine
label-gender-stork = feminine
label-gender-swan = masculine
label-gender-tiger = masculine
label-gender-snake = masculine
label-gender-toucan = masculine
label-gender-turtle = feminine
label-gender-walrus = masculine
label-gender-whale = feminine
label-gender-wolf = masculine
label-gender-woodpecker = masculine
label-gender-yak = masculine
label-gender-zebra = masculine
label-gender-dog = masculine
label-gender-cow = feminine
label-gender-beaver = masculine

# Translators: the laptop's name is a colour plus an animal, for example "Blue Fox".
# French: animal first, colour after, agreeing with the animal ("Renard bleu").
label-picked = { $animal } { $colour }
label-unnamed = Ordinateur portable sans nom

## M02 Finding your files

scan-eyebrow = Votre transfert
scan-title = Recherche de vos fichiers
scan-body = Ce que PCTwin trouve est enregistré, pour que la recherche puisse reprendre si elle s’arrête.
scan-progress =
    .aria-label = Recherche dans les endroits choisis
# $size is passed but not shown in the mockup line.
scan-found =
    { NUMBER($count) ->
        [one] { $count } fichier trouvé pour l’instant.
        [many] { $count } de fichiers trouvés pour l’instant.
       *[other] { $count } fichiers trouvés pour l’instant.
    }
scan-step-usual-title = Fichiers et dossiers
scan-step-usual-body = Recherche faite dans les endroits que PCTwin peut lire.
scan-step-other-title = Autres dossiers et disques
scan-step-other-body = Les dossiers et disques que vous utilisez en dehors des endroits habituels.
scan-step-protected-title = Endroits protégés
scan-step-protected-body =
    { NUMBER($count) ->
        [one] { $count } endroit attend encore votre autorisation.
        [many] { $count } d’endroits attendent encore votre autorisation.
       *[other] { $count } endroits attendent encore votre autorisation.
    }
scan-step-done = Terminé
scan-step-current = En cours
scan-step-todo = À venir
scan-unreadable-note = Les endroits que PCTwin n’a pas pu lire restent dans la liste. Ils ne comptent pas comme recherchés.
scan-status-not-searched = Non recherché
scan-paused = Recherche en pause. Tout ce qui a été trouvé est enregistré.
scan-resumed = La recherche a repris là où elle s’était arrêtée.
scan-partial =
    { NUMBER($count) ->
        [one] { $count } endroit protégé n’a pas encore été recherché.
        [many] { $count } d’endroits protégés n’ont pas encore été recherchés.
       *[other] { $count } endroits protégés n’ont pas encore été recherchés.
    }
scan-allow = Autoriser l’accès
scan-pause = Pause
scan-resume = Reprendre la recherche
scan-continue = Voir ce qui a été trouvé

## M03 Everything you chose, in view

overview-eyebrow = Votre transfert
overview-title = Ce que PCTwin a trouvé
overview-body = Vérifiez ce que PCTwin a trouvé avant qu’il prépare le plan.
# The metric value next to each label is passed in already formatted.
overview-size-label = fichiers sélectionnés
overview-items-label =
    { NUMBER($count) ->
        [one] fichier
        [many] de fichiers
       *[other] fichiers
    }
overview-attention-label =
    { NUMBER($count) ->
        [one] demande votre attention
        [many] demandent votre attention
       *[other] demandent votre attention
    }
overview-files-title = Fichiers et dossiers
overview-files-body = Documents, photos et autres dossiers choisis.
overview-dest-title = Destination
overview-dest-body = L’endroit sur le nouvel ordinateur portable pour chaque dossier.
overview-people-title = Personnes
overview-people-body = Choisissez le compte de chaque personne sur le nouvel ordinateur portable.
overview-review = Vérifier
overview-partial =
    { NUMBER($count) ->
        [one] { $count } endroit protégé n’a pas encore été recherché.
        [many] { $count } d’endroits protégés n’ont pas encore été recherchés.
       *[other] { $count } endroits protégés n’ont pas encore été recherchés.
    }
overview-allow = Autoriser l’accès
overview-continue = Continuer

## M04 Keep what matters to you

files-eyebrow = Votre transfert
files-title = Choisissez les dossiers à transférer
files-body = Choisissez des dossiers, d’autres disques et tout autre endroit.
files-search-label = Trouver un dossier
files-search-placeholder = Rechercher des dossiers
files-select =
    .aria-label = Transférer { $name }
files-folder-detail =
    { NUMBER($count) ->
        [one] { $size } · { $count } fichier
        [many] { $size } · { $count } de fichiers
       *[other] { $size } · { $count } fichiers
    }
files-status-protected = Autorisation nécessaire
files-protected-body = PCTwin a besoin de votre autorisation pour lire ce dossier. Ses fichiers ne sont pas comptés tant que vous ne l’avez pas autorisé.
files-allow = Autoriser l’accès
files-status-added = Ajouté par vous
files-add = Ajouter un autre dossier
files-remove =
    .aria-label = Retirer { $name }
files-no-limit = Les transferts gratuits n’ont aucune limite de taille de fichier ni de nombre de dossiers.
files-total =
    { NUMBER($count) ->
        [one] { $count } dossier choisi, { $size } au total
        [many] { $count } de dossiers choisis, { $size } au total
       *[other] { $count } dossiers choisis, { $size } au total
    }
files-none = Choisissez au moins un dossier pour continuer.
files-continue = Choisir les destinations

## M05 Give every file a home

dest-eyebrow = Votre transfert
dest-title = Où vont vos dossiers
dest-body = Choisissez où va chaque dossier sur le nouvel ordinateur portable.
dest-col-chosen = Votre choix
dest-col-place = Destination
dest-col-placement = Disque
dest-placement-local = Interne
dest-placement-external = Externe
dest-col-status = État
dest-select =
    .aria-label = Destination de { $name }
# $folder is a folder name on the new laptop, for example "Documents".
dest-option-folder = Nouvel ordinateur portable / { $folder }
# $drive is the name of a drive connected to the new laptop.
dest-option-drive = Disque externe / { $drive }
dest-option-choose = Choisir un dossier
dest-default-label = Endroit habituel
dest-status-ready = Prêt
dest-status-missing = Disque absent
dest-status-space = Espace insuffisant
dest-all-ready = Chaque dossier a une place sur le nouvel ordinateur portable.
dest-missing = { $drive } n’est pas connecté au nouvel ordinateur portable. Connectez-le, ou choisissez un autre endroit pour ces fichiers.
dest-space = { $place } a { $free } d’espace libre, mais ces fichiers ont besoin de { $needed }. Libérez de l’espace, choisissez un autre endroit ou transférez moins de fichiers.
dest-change = Modifier
dest-continue = Vérifier le plan

## M23 Approve the move you want

plan-eyebrow = Votre transfert
plan-title = Vérifiez et approuvez le plan
plan-body = PCTwin fait seulement ce que ce plan indique.
plan-what-title = Votre choix
# $size is already formatted, for example "120 GB". $count is passed but not shown.
plan-what-body = { $size } de fichiers, depuis les dossiers choisis, vers les endroits indiqués.
plan-where-body =
    { NUMBER($count) ->
        [one] Vers { $count } endroit sur le nouvel ordinateur portable.
        [many] Vers { $count } d’endroits sur le nouvel ordinateur portable.
       *[other] Vers { $count } endroits sur le nouvel ordinateur portable.
    }
plan-hand-title = Avant de commencer
plan-hand-body =
    { NUMBER($count) ->
        [one] { $count } étape. Elle figure dans votre rapport à la fin.
        [many] { $count } d’étapes. Elles figurent dans votre rapport à la fin.
       *[other] { $count } étapes. Elles figurent dans votre rapport à la fin.
    }
plan-hand-none = Rien, d’après ce que PCTwin peut voir maintenant.
plan-old-safe = Votre ancien ordinateur portable reste inchangé. Les fichiers sont copiés, jamais retirés.
plan-edit = Modifier le plan
plan-edit-note = Si vous modifiez le plan, vous l’approuvez à nouveau avant que quelque chose de nouveau commence.
plan-approve = J’approuve ce plan, y compris ce que je dois faire moi-même.
plan-not-approved = Vous n’avez pas encore approuvé ce plan, donc rien ne sera transféré.
plan-changed = Vous avez modifié le plan : approuvez-le à nouveau avant tout nouveau transfert.
plan-start = Commencer le transfert

## Plan screens: shared words and the lines the copy deck did not cover

plan-back = Retour
# $n is a count of files or folders.
plan-number = { NUMBER($n) }
# $n is a number already rounded for reading, for example 18.4.
plan-size-b = { NUMBER($n) } octets
plan-size-kb = { NUMBER($n) } KB
plan-size-mb = { NUMBER($n) } MB
plan-size-gb = { NUMBER($n) } GB
plan-size-tb = { NUMBER($n) } TB
# $size is a size already written out, for example "18.4 GB". Used when part of a folder could not be read.
plan-size-at-least = au moins { $size }
plan-size-unknown = taille inconnue
plan-role-home = Dossier personnel
plan-role-desktop = Bureau
plan-role-documents = Documents
plan-role-downloads = Téléchargements
plan-role-pictures = Images
plan-role-music = Musique
plan-role-videos = Vidéos
plan-role-public = Dossier public
plan-role-app-data = Données des applis
plan-role-other = Autre dossier
# $done and $total are counts of places this search has looked at and has to look at.
scan-places = Endroits recherchés : { $done } sur { $total }
scan-card-title = Recherche dans les endroits choisis
scan-pausing = Pause après le dossier en cours. Tout ce qui a été trouvé est enregistré.
overview-row-attention = Attention nécessaire
files-status-work = Travail ou école
files-work-body = Votre entreprise ou votre école gère ce dossier, il n’est donc pas inclus sauf si vous le cochez.
# $count is how many files in the folder are only online.
files-online-only =
    { NUMBER($count) ->
        [one] { $count } fichier est seulement en ligne, il n’est donc pas copié.
        [many] { $count } de fichiers sont seulement en ligne, ils ne sont donc pas copiés.
       *[other] { $count } fichiers sont seulement en ligne, ils ne sont donc pas copiés.
    }
files-no-match = Aucun dossier ne porte ce nom.
dest-status-none = Choisir un endroit
dest-status-readonly = Écriture impossible
dest-status-format = Fichier trop volumineux
# $name is the folder that needs a place.
dest-none = Choisissez un endroit pour { $name } sur le nouvel ordinateur portable.
# $place is a drive on the new laptop that cannot be written to.
dest-readonly = { $place } est en lecture seule, rien ne peut donc y être copié. Choisissez un autre endroit.
# $place is the drive. Some drive formats cannot hold a single very large file.
dest-format = Un fichier de ces dossiers est trop volumineux pour la façon dont { $place } est configuré. Choisissez un autre endroit.
# $folder is a folder on the new laptop and $drive the drive it is on.
plan-place = { $folder } sur { $drive }
plan-group-to = Vers { $place }
plan-group-move = Déplacer { $folder }
plan-place-change = Choisir un autre endroit
plan-place-usual = Utiliser l'endroit habituel
plan-place-free = { $size } libres à cet endroit
plan-place-no-room = Il n'y a pas assez d'espace libre à cet endroit. Choisissez un autre endroit, ou décochez ce dossier.
plan-place-no-safe-name = PCTwin ne peut pas enregistrer de fichiers en toute sécurité sur ce disque. Choisissez un autre endroit.
plan-place-cannot-open = PCTwin n'a pas pu ouvrir ce dossier. Choisissez un autre endroit.
plan-place-locked = Ce disque est verrouillé. Déverrouillez-le d’abord, puis choisissez-le à nouveau.
plan-version = Version du plan { $revision }
plan-details = Plus de détails
plan-preview-title = Avant de commencer
plan-preview-fits = Ce portable a assez d’espace libre.
plan-preview-fits-about = Ce portable devrait avoir assez d’espace libre. Environ { $extra } sont comptés pour les applis que PCTwin installe et leurs données.
# Added 23 October 2026: what moves first, from the move's own order. $places is a list of folder names.
plan-preview-first = Ces éléments sont transférés en premier, pour que vous puissiez utiliser votre nouvel ordinateur portable plus tôt : { $places }.
plan-preview-short = Ce portable a besoin de { $size } d’espace libre en plus. Libérez de l’espace avant de commencer, sinon le transfert s’arrêtera quand le disque sera plein.
plan-preview-short-about = Ce portable a besoin d’environ { $size } d’espace libre en plus, en comptant environ { $extra } pour les applis que PCTwin installe et leurs données. Libérez de l’espace avant de commencer, sinon le transfert s’arrêtera quand le disque sera plein.
plan-preview-space-unknown = PCTwin n’a pas pu lire l’espace libre de ce portable.
plan-preview-synced = Sur ce portable, { $folder } est conservé dans { $service }. Les fichiers transférés là seront envoyés vers { $service } et utiliseront votre espace là-bas.
plan-preview-time-one =
    { NUMBER($minutes) ->
        [one] Environ { $minutes } minute.
        [many] Environ { $minutes } de minutes.
       *[other] Environ { $minutes } minutes.
    }
plan-preview-time = Environ { $low } à { $high } minutes.
plan-preview-renamed =
    { NUMBER($count) ->
        [one] { $count } fichier aura un nom un peu différent, car ce portable n’accepte pas le sien. Le rapport l’indique.
        [many] { $count } de fichiers auront un nom un peu différent, car ce portable n’accepte pas les leurs. Le rapport les indique un par un.
       *[other] { $count } fichiers auront un nom un peu différent, car ce portable n’accepte pas les leurs. Le rapport les indique un par un.
    }
plan-preview-too-big =
    { NUMBER($count) ->
        [one] { $count } fichier est trop gros pour le disque de ce portable et ne sera pas copié.
        [many] { $count } de fichiers sont trop gros pour le disque de ce portable et ne seront pas copiés.
       *[other] { $count } fichiers sont trop gros pour le disque de ce portable et ne seront pas copiés.
    }
plan-preview-refused =
    { NUMBER($count) ->
        [one] { $count } fichier a un nom ou un dossier que ce portable n’accepte pas, et ne sera pas copié.
        [many] { $count } de fichiers ont un nom ou un dossier que ce portable n’accepte pas, et ne seront pas copiés.
       *[other] { $count } fichiers ont un nom ou un dossier que ce portable n’accepte pas, et ne seront pas copiés.
    }
plan-preview-runs-at-sign-in =
    { NUMBER($count) ->
        [one] { $count } fichier se lancerait tout seul à l’ouverture d’une session sur ce portable. Vérifiez que c’est voulu.
        [many] { $count } de fichiers se lanceraient tout seuls à l’ouverture d’une session sur ce portable. Vérifiez que c’est voulu.
       *[other] { $count } fichiers se lanceraient tout seuls à l’ouverture d’une session sur ce portable. Vérifiez que c’est voulu.
    }
# $reference is a short code that names exactly this version of the plan.
plan-reference = Référence { $reference }
# $count is how many files are affected in each line below.
plan-hand-cloud-only =
    { NUMBER($count) ->
        [one] { $count } fichier est seulement en ligne. PCTwin ne le télécharge pas. Rendez-le d’abord disponible sur l’ancien ordinateur portable, ou laissez-le.
        [many] { $count } de fichiers sont seulement en ligne. PCTwin ne les télécharge pas. Rendez-les d’abord disponibles sur l’ancien ordinateur portable, ou laissez-les.
       *[other] { $count } fichiers sont seulement en ligne. PCTwin ne les télécharge pas. Rendez-les d’abord disponibles sur l’ancien ordinateur portable, ou laissez-les.
    }
plan-hand-unreadable =
    { NUMBER($count) ->
        [one] { $count } élément n’a pas pu être ouvert, il n’est donc pas transféré. Autorisez l’accès, ou laissez-le.
        [many] { $count } d’éléments n’ont pas pu être ouverts, ils ne sont donc pas transférés. Autorisez l’accès, ou laissez-les.
       *[other] { $count } éléments n’ont pas pu être ouverts, ils ne sont donc pas transférés. Autorisez l’accès, ou laissez-les.
    }
plan-hand-work-managed =
    { NUMBER($count) ->
        [one] { $count } élément est géré par votre entreprise ou votre école et n’est pas inclus. Cochez son dossier pour l’inclure.
        [many] { $count } d’éléments sont gérés par votre entreprise ou votre école et ne sont pas inclus. Cochez leurs dossiers pour les inclure.
       *[other] { $count } éléments sont gérés par votre entreprise ou votre école et ne sont pas inclus. Cochez leurs dossiers pour les inclure.
    }
plan-hand-too-many-files =
    { NUMBER($count) ->
        [one] { $count } dossier contient plus de fichiers qu’un seul transfert ne peut en emporter, il n’est donc pas déplacé. Après ce transfert, envoyez-le à part avec Envoi rapide, ou copiez-le avec une clé USB.
        [many] { $count } de dossiers contiennent plus de fichiers qu’un seul transfert ne peut en emporter, ils ne sont donc pas déplacés. Après ce transfert, envoyez chacun à part avec Envoi rapide, ou copiez-les avec une clé USB.
       *[other] { $count } dossiers contiennent plus de fichiers qu’un seul transfert ne peut en emporter, ils ne sont donc pas déplacés. Après ce transfert, envoyez chacun à part avec Envoi rapide, ou copiez-les avec une clé USB.
    }
plan-blocked-someone-unmapped = Une personne de l’ancien ordinateur portable n’a pas encore de compte choisi sur le nouveau. Choisissez où vont les fichiers de chaque personne, puis revenez.
plan-blocked-account-used-twice = Deux personnes se retrouveraient dans le même compte sur le nouvel ordinateur portable. Donnez à chacune son propre compte.
plan-blocked-invalid = Ce plan ne peut pas être approuvé tel quel. Revenez en arrière et vérifiez vos choix.
plan-blocked-fix = Choisir les comptes

## Tray menu

tray-show = Afficher PCTwin
tray-send = Envoyer des fichiers
tray-quit = Quitter PCTwin

## F01 secondary actions and their coming-soon pages

welcome-restore = Restaurer une configuration
welcome-rescue = Sauver un ordinateur portable
welcome-quick-send = Envoi rapide

soon-eyebrow = Bientôt disponible
soon-restore-title = Restaurer une configuration
soon-restore-body = Reconstruire un ordinateur portable à partir d’une configuration enregistrée après un transfert. Cette fonction arrive dans une prochaine version.
soon-rescue-title = Sauver un ordinateur portable
soon-rescue-body = Récupérer les fichiers d’un ordinateur portable qui ne démarre plus, en branchant son disque sur un ordinateur qui fonctionne. Cette fonction arrive dans une prochaine version.
soon-move-title = Transfert complet
soon-move-body = Transférez tout de votre ancien ordinateur portable vers le nouveau : vos fichiers, vos applications et vos réglages. Cette fonction arrive dans une prochaine version. En attendant, utilisez l’Envoi rapide pour envoyer des fichiers.

## X01 Move progress, compact

progress-eyebrow = Votre transfert
progress-title = Transfert de vos affaires vers cet ordinateur portable
progress-title-old = Envoi de vos affaires vers votre nouveau portable
progress-title-paused = Transfert en pause
progress-paired = Associés
progress-percent =
    .aria-label = Progression du transfert
# $done and $total are already formatted sizes, for example "81.6 GB" and "120 GB".
progress-amount = { $done } sur { $total }
# $range is a measured range, already formatted, for example "15 to 20 minutes".
progress-time-left = Encore environ { $range }
progress-time-measuring = Calcul du temps restant…
# $speed is measured and already formatted, for example "62 MB/s".
progress-speed = { $speed }, mesuré
progress-count-needs-action =
    { NUMBER($count) ->
        [one] { $count } élément demande votre action
        [many] { $count } d’éléments demandent votre action
       *[other] { $count } éléments demandent votre action
    }
progress-current-copying = Copie en cours : { $name }
progress-current-checking = Vérification en cours : { $name }
progress-needs-action =
    { NUMBER($count) ->
        [one] { $count } élément demande votre action. Le reste continue.
        [many] { $count } d’éléments demandent votre action. Le reste continue.
       *[other] { $count } éléments demandent votre action. Le reste continue.
    }
progress-see-needs-action = Voir les actions à faire
progress-interrupted = La connexion avec l’autre ordinateur portable a été coupée. PCTwin essaie de se reconnecter. Rien de ce qui a été copié et vérifié n’est perdu.
progress-low-battery = La batterie de ce portable est faible. Branchez-le pour que le transfert ne s’interrompe pas.
progress-battery-paused = La batterie de ce portable est presque vide : le transfert est en pause, en toute sécurité. Branchez-le, puis continuez. Rien n’est perdu.
progress-old-safe = Votre ancien ordinateur portable reste inchangé.
progress-pause = Mettre en pause
progress-compact = Afficher moins
progress-expand = Afficher tous les éléments
progress-stop = Arrêter le transfert

## X02 Move progress, expanded

progress-all-title = Tous les éléments du transfert
progress-all-body = Chaque élément indique d’où il vient et où il en est.
progress-metric-overall = progression globale
progress-metric-committed = copiés et vérifiés
progress-metric-estimate = estimation
progress-col-item = Élément
progress-col-from = Origine
progress-col-status = État
progress-state-waiting = En attente
progress-state-copying = Copie
progress-state-verified = Vérifié
progress-state-checking = Vérification
progress-state-paused = En pause
progress-checked-note = Un élément ne compte comme copié qu’une fois vérifié.
# Added 23 October 2026: T14, one tip under the progress bar, about every 12 seconds.
tips-label = Conseil
tips-pause = Suspendre les conseils
tips-resume = Afficher d’autres conseils
tips-next = Conseil suivant
tip-battery = Ce portable fonctionne sur batterie. Branchez-le si vous le pouvez, pour qu’un long transfert ne soit pas interrompu.
tip-closer = La connexion est lente. Rapprocher les ordinateurs portables du routeur Wi-Fi peut l’accélérer.
tip-lids = Gardez l’écran de ce portable ouvert. Un écran fermé peut le mettre en veille, ce qui met le transfert en pause.
tip-keep-using = Vous pouvez continuer à utiliser ce portable. PCTwin ne fait que lire ses fichiers.
tip-pause = Vous pouvez mettre le transfert en pause à tout moment et reprendre plus tard.
tip-power-cut = En cas de coupure de courant, rallumez les deux ordinateurs portables et ouvrez PCTwin. Il reprend là où il s’était arrêté, et rien de ce qui a été copié et vérifié n’est perdu.

## F09 Mini panel

mini-title = Transfert en cours
mini-title-paused = Transfert en pause
mini-what = transfert de vos fichiers sélectionnés
mini-progress =
    .aria-label = Progression du transfert
mini-pause = Pause
mini-resume = Reprendre
mini-open = Ouvrir le transfert
# Used when lane E reuses this panel for a Quick Send.
mini-title-send = Envoi des fichiers
mini-what-send = envoi des fichiers sélectionnés
mini-open-send = Ouvrir l’envoi
mini-progress-send =
    .aria-label = Progression de l’envoi

## X05 Paused at a safe point

pause-title = Le transfert est en pause
pause-title-finishing = Mise en pause du transfert…
pause-body = Tout ce qui a été copié et vérifié est gardé. Le reste attend.
pause-finishing = PCTwin termine d’abord { $name }, pour ne pas le laisser à moitié copié.
pause-kept-title = Fichiers
pause-kept-body = Tout ce qui a été copié et vérifié jusqu’ici est gardé.
pause-waiting-title = En attente
pause-waiting-body =
    { NUMBER($count) ->
        [one] { $count } élément reste à copier. Rien n’est perdu.
        [many] { $count } d’éléments restent à copier. Rien n’est perdu.
       *[other] { $count } éléments restent à copier. Rien n’est perdu.
    }
pause-status-kept = En pause
pause-status-waiting = En attente
pause-note = La pause n’annule rien. Votre ancien ordinateur portable reste inchangé.
pause-resume = Reprendre le transfert

## X10 Stop the remaining move? (danger dialog)

cancel-title = Arrêter le reste du transfert ?
cancel-body = L’arrêt garde tout ce qui a été copié et vérifié. Il n’annule pas le transfert.
cancel-copied-title = Fichiers copiés
cancel-copied-body = Restent sur le nouvel ordinateur portable.
cancel-rest-title = Le reste du transfert
cancel-rest-body = Rien de nouveau ne commence. Les fichiers pas encore copiés restent seulement sur l’ancien ordinateur portable.
cancel-partial-title = Un fichier copié en partie
cancel-partial-body = N’est jamais gardé comme s’il était complet.
cancel-old-title = Ancien ordinateur portable
cancel-old-body = Reste inchangé.
cancel-ack = Je comprends que l’arrêt n’annule pas ce qui a été copié
cancel-stop = Arrêter et voir le rapport
cancel-keep = Continuer le transfert

## R01 Welcome to your next laptop (outcome)

outcome-eyebrow = Résultats du transfert
outcome-title = Le transfert est terminé
outcome-body = Voici ce qui est arrivé à tout ce que vous avez approuvé.
outcome-title-old = Le transfert est terminé
outcome-body-old = Voici ce qui est arrivé à tout ce que vous avez envoyé. Cet ordinateur reste tel qu'il était.
outcome-files-title = Fichiers sélectionnés
outcome-status-partial = Fait en partie
# Added 23 October 2026: the after-move check on R01, counted the way the scan counts.
outcome-kinds-title = Vérifié après le transfert
outcome-kind-photos = Photos
outcome-kind-videos = Vidéos
outcome-kind-music = Musique
outcome-kind-documents = Documents
outcome-kind-other = Autres fichiers
outcome-kind-count = { $copied } sur { $planned } copiés et vérifiés
outcome-next-title = Que faire ensuite
outcome-old-safe = Tout ce qui n’est pas terminé reste listé ici. Rien n’a été retiré de votre ancien ordinateur portable.
outcome-landed-title = Où se trouvent maintenant vos affaires
outcome-report = Voir le rapport complet
outcome-home = Retour à l’accueil
outcome-person-copied =
    { NUMBER($count) ->
        [one] { $count } fichier copié.
        [many] { $count } de fichiers copiés.
       *[other] { $count } fichiers copiés.
    }
outcome-person-not-done =
    { NUMBER($count) ->
        [one] { $count } n’a pas abouti.
        [many] { $count } de fichiers n’ont pas abouti.
       *[other] { $count } n’ont pas abouti.
    }
outcome-person-at-sign-in = Elles seront en place à la prochaine connexion de { $name } sur ce portable.
move-fallback-folder = De votre ancien portable
move-stop = Arrêter le transfert
move-before-open = En attendant : avant de quitter ce portable
move-get-help = Obtenir de l’aide
pickup-eyebrow = Votre transfert
pickup-title = Remettre en ordre le transfert interrompu
pickup-body = Un transfert sur ce portable s’est arrêté en cours de route, peut-être après un redémarrage ou une coupure de courant. Faites d’abord le tri, puis associez à nouveau les portables pour continuer.
pickup-unfinished-title = Copies inachevées
pickup-unfinished-body =
    { NUMBER($count) ->
        [one] { $count } fichier était en cours de copie quand le transfert s’est arrêté.
        [many] { $count } de fichiers étaient en cours de copie quand le transfert s’est arrêté.
       *[other] { $count } fichiers étaient en cours de copie quand le transfert s’est arrêté.
    }
pickup-unfinished-badge = À trier
pickup-tidy = Faire le tri
pickup-tidying = Tri en cours. Les copies terminées sont vérifiées et mises en place ; rien de ce qui est à vous n’est modifié.
pickup-done = La remise en ordre est terminée. Associez de nouveau les ordinateurs portables pour continuer : ce qui est déjà ici est retrouvé et n’est pas copié deux fois.
pickup-failed = Le tri n’a pas fonctionné cette fois. Rien de ce qui est à vous n’a changé. Réessayez, ou fermez d’abord PCTwin sur ce portable puis rouvrez-le.
pickup-put-in-place = mis en place
pickup-kept = gardés pour continuer
pickup-not-finished =
    { NUMBER($count) ->
        [one] { $count } fichier n’a pas pu être terminé. Le prochain transfert le recopiera.
        [many] { $count } de fichiers n’ont pas pu être terminés. Le prochain transfert les recopiera.
       *[other] { $count } fichiers n’ont pas pu être terminés. Le prochain transfert les recopiera.
    }
pickup-left-for-later =
    { NUMBER($count) ->
        [one] { $count } fichier est sur un disque débranché pour l’instant. Branchez-le et refaites le tri.
        [many] { $count } de fichiers sont sur un disque débranché pour l’instant. Branchez-le et refaites le tri.
       *[other] { $count } fichiers sont sur un disque débranché pour l’instant. Branchez-le et refaites le tri.
    }
pickup-later = Pas maintenant
ready-eyebrow = Depuis votre ancien portable
ready-title = Vos affaires sont dans vos dossiers
ready-body = Vos affaires de l’ancien portable ont été déplacées sur celui-ci et sont maintenant dans vos propres dossiers.
ready-landed = fichiers dans vos dossiers
ready-where-title = Où elles sont
ready-where-body = Dans Documents, Images, Musique, Vidéos et vos autres dossiers habituels, comme sur votre ancien portable.
ready-kept =
    { NUMBER($count) ->
        [one] { $count } fichier a été mis de côté, car un fichier de ce nom était déjà là. Les deux sont gardés.
        [many] { $count } de fichiers ont été mis de côté, car des fichiers de ces noms étaient déjà là. Tous sont gardés.
       *[other] { $count } fichiers ont été mis de côté, car des fichiers de ces noms étaient déjà là. Tous sont gardés.
    }
ready-failed =
    { NUMBER($count) ->
        [one] { $count } fichier n’a pas pu être mis dans vos dossiers. Demandez à la personne qui a fait le transfert de le refaire.
        [many] { $count } de fichiers n’ont pas pu être mis dans vos dossiers. Demandez à la personne qui a fait le transfert de les refaire passer.
       *[other] { $count } fichiers n’ont pas pu être mis dans vos dossiers. Demandez à la personne qui a fait le transfert de les refaire passer.
    }
ready-done = Terminé
move-status-kept = Conservé
move-wait-eyebrow = Votre transfert
move-wait-scanning-title = Recherche de vos fichiers
move-wait-scanning-body = Votre ancien portable parcourt vos dossiers. Il les lit seulement : rien n’est ouvert, modifié ou supprimé.
move-found = Trouvé jusqu’ici : { $size } de vos affaires.
move-wait-approval-title = En attente du nouveau portable
move-wait-approval-body = Vos fichiers sont prêts. Vérifiez le plan sur le nouveau portable et approuvez-le pour commencer.
move-wait-list-title = En attente de l’ancien portable
move-wait-list-body = Votre ancien ordinateur portable cherche vos affaires. Ensuite, ce qui sera transféré y est choisi. Quand c’est fait, la liste apparaît ici pour que vous l’acceptiez. Cela peut prendre quelques minutes.
move-wait-starting-title = Début du transfert
move-wait-starting-body = Le plan est approuvé. La copie commence dans un instant.
move-wait-admin-title = En attente d’un administrateur
move-wait-admin-body = Ce portable demande l’accord d’un administrateur pour déplacer aussi les affaires des autres personnes. Répondez dans la fenêtre qu’il a ouverte.
move-others-not-approved = L’administrateur n’a pas donné son accord : seules vos affaires sont déplacées cette fois. Celles des autres restent en place.
move-others-failed = PCTwin n’a pas pu lancer ce qu’il faut pour déplacer les affaires des autres : seules les vôtres sont déplacées cette fois. Rien des leurs n’a changé.
move-others-not-here = Déplacer les affaires d’autres personnes depuis ce portable n’est pas encore prêt dans cette version : seules les vôtres sont déplacées.
move-others-allow-in-settings = Un administrateur doit d’abord autoriser PCTwin dans la page des Réglages Système qui vient de s’ouvrir. Cette fois, seules vos affaires sont déplacées ; une fois PCTwin autorisé, votre prochain déplacement pourra prendre aussi les affaires des autres personnes.
approve-prompt = PCTwin va lire les fichiers de { $names } pour les déplacer vers le nouveau portable. Un administrateur de ce portable donne son accord pour ce déplacement seulement.
who-here-title = Qui déménage ?
who-here-body = Vos affaires sont toujours déplacées. Cochez toute autre personne de ce portable dont les affaires doivent l’être aussi. PCTwin ne vous montre jamais leurs fichiers.
who-here-you = { $name } (vous)
who-here-you-hint = Toujours déplacé
who-here-approval = Déplacer les affaires de quelqu’un d’autre demande l’accord d’un administrateur, une seule fois pour toutes les personnes cochées. Le portable le demandera dans sa propre fenêtre.
who-here-not-here = Déplacer les affaires d’autres personnes depuis ce portable n’est pas encore prêt dans cette version. Seules les vôtres sont déplacées.
place-title = Où vont leurs affaires ?
place-body = Les affaires d’autres personnes viennent aussi de l’ancien portable. Choisissez pour chacune un compte de ce portable, un nouveau compte, ou laissez-la de côté. Vous ne voyez jamais leurs fichiers.
place-size =
    { NUMBER($count) ->
        [one] { $count } fichier, { $size }
        [many] { $count } de fichiers, { $size }
       *[other] { $count } fichiers, { $size }
    }
place-select = Où vont les affaires de { $name }
place-note = Les nouveaux comptes, et la garde de leurs affaires jusqu’à leur connexion, demandent l’accord d’un administrateur une seule fois, après votre accord sur le plan. Le mot de passe temporaire de chaque nouveau compte s’affiche une fois, dans une fenêtre du portable lui-même.
place-todo =
    { NUMBER($count) ->
        [one] Choisissez où vont les affaires de { $count } personne de plus.
        [many] Choisissez où vont les affaires de { $count } de personnes de plus.
       *[other] Choisissez où vont les affaires de { $count } personnes de plus.
    }
fda-title = PCTwin a besoin d’une autorisation pour lire certains dossiers
fda-body =
    { NUMBER($count) ->
        [one] { $count } emplacement n’a pas pu être lu à cause des réglages de confidentialité du Mac. Donnez l’accès à PCTwin, puis vérifiez à nouveau, pour qu’aucun de vos fichiers ne soit oublié.
        [many] { $count } emplacements n’ont pas pu être lus à cause des réglages de confidentialité du Mac. Donnez l’accès à PCTwin, puis vérifiez à nouveau, pour qu’aucun de vos fichiers ne soit oublié.
       *[other] { $count } emplacements n’ont pas pu être lus à cause des réglages de confidentialité du Mac. Donnez l’accès à PCTwin, puis vérifiez à nouveau, pour qu’aucun de vos fichiers ne soit oublié.
    }
fda-step-1 = Ouvrez Réglages Système, puis Confidentialité et sécurité.
fda-step-2 = Choisissez Accès complet au disque.
fda-step-3 = Activez PCTwin. Le Mac peut demander votre mot de passe.
fda-open = Ouvrir ces réglages
fda-note = PCTwin lit seulement vos dossiers pour les copier. Il ne modifie jamais rien sur ce portable.
fda-again = Vérifier à nouveau
fda-without = Continuer sans eux
password-title = PCTwin : un nouveau compte
password-body =
    { $name } a maintenant un compte sur ce portable.
    
    Nom de connexion : { $account }
    Mot de passe temporaire : { $password }
    
    Donnez-les à { $name }. À sa première connexion, la personne choisit son propre mot de passe. Notez-le maintenant : PCTwin ne le garde pas.
move-ended-eyebrow = Votre transfert
move-ended-kept-title = Déjà copié
move-ended-kept-body = Tout ce qui a été copié et vérifié avant cela reste sur le nouveau portable.
move-ended-old-title = Votre ancien portable
move-ended-old-body = Rien n’y a été modifié ni supprimé.
move-ended-old-badge = Inchangé
move-ended-next = Pour réessayer, associez à nouveau les portables. Les fichiers déjà copiés et vérifiés sont reconnus et ne sont pas copiés deux fois.
move-lost-title = La connexion avec votre autre portable a été perdue
move-lost-body = Les portables ne communiquent plus, le transfert n’a donc pas pu continuer. Vérifiez qu’ils sont allumés, éveillés et proches l’un de l’autre.
move-not-approved-title = Le transfert n’a pas commencé
move-not-approved-old = Le plan n’a pas été approuvé sur le nouveau portable, rien n’a donc été copié.
move-not-approved-new = Vous n’avez pas approuvé le plan, rien n’a donc été copié.
move-failed-title = Le transfert s’est arrêté à cause d’une erreur
move-failed-body = Le transfert n’a pas pu continuer. Si cela se reproduit, Plus de détails, ci-dessous, indique ce qui s’est passé.
move-peer-system-windows = Windows
move-peer-system-mac = Mac
move-peer-system-linux = Linux
move-peer-system-other = Autre système
move-update-here-title = Mettez à jour PCTwin sur cet ordinateur
move-update-here-body = Cet ordinateur a une version de PCTwin plus ancienne que l'autre, ils ne peuvent donc pas transférer de fichiers ensemble. Mettez à jour PCTwin ici, puis recommencez. Rien n'a été copié.
move-update-there-title = Mettez à jour PCTwin sur l'autre ordinateur
move-update-there-body = L'autre ordinateur a une version de PCTwin plus ancienne que celui-ci, ils ne peuvent donc pas transférer de fichiers ensemble. Mettez à jour PCTwin sur l'autre, puis recommencez. Rien n'a été copié.
move-refused-title = Le transfert n’a pas pu commencer
move-refused-not-paired = Vos portables ne sont plus associés. Associez-les à nouveau pour commencer le transfert.
move-refused-busy = Un transfert est déjà en cours sur ce portable.
move-refused-not-moving = Aucun transfert n’est en cours sur ce portable.
move-refused-plan-changed = Le plan a changé avant d’être approuvé. Associez à nouveau les portables pour voir le nouveau plan.
move-refused-failed = Les dossiers de ce portable n’ont pas pu être préparés. Associez à nouveau les portables pour réessayer.
drive-problem-title = Le transfert a besoin de vous
drive-problem-body = Les copies enregistrées dans { $place } ne correspondaient pas aux originaux lors des vérifications de PCTwin, plusieurs fois. Le disque est peut-être défaillant, plein ou débranché : la copie y est en pause.
drive-problem-kept-title = Déjà copié et vérifié
drive-problem-waiting-title = Reste à copier dans { $place }
drive-problem-badge = En pause
drive-problem-fix = Vérifiez que le disque est branché et a de l’espace libre, puis continuez. Rien de ce que vous avez choisi n’est écarté pour contourner le problème. Vous pouvez aussi arrêter le transfert.
drive-problem-carry-on = Continuer

## R02 Your move, item by item (detailed report)

report-title = Ce qui a été déplacé
report-body = Sélectionnez un groupe pour voir ce qu’il contient. Ce qui a besoin de vous est en haut.
report-needs-title = { $count ->
        [one] À faire par vous : { $count }
        [many] À faire par vous : { $count }
       *[other] À faire par vous : { $count }
    }
report-group-photos = Photos
report-group-videos = Vidéos
report-group-music = Musique
report-group-documents = Documents
report-group-other = Autres fichiers
report-group-size = { $files ->
        [one] { $files } fichier
        [many] { $files } fichiers
       *[other] { $files } fichiers
    } · { $size }
report-group-done = Tout est copié et vérifié
report-group-some = { $done } sur { $total } copiés et vérifiés
report-group-left = { $count ->
        [one] { $count } non déplacé : vous l’avez laissé de côté
        [many] { $count } non déplacés : vous les avez laissés de côté
       *[other] { $count } non déplacés : vous les avez laissés de côté
    }
report-filter-label = Afficher
report-col-item = Élément
report-col-result = Résultat
report-col-details = Vérification ou étape suivante
report-from = De : { $path }
report-to = Vers : { $path }
report-none = Aucun élément n’a ce résultat.
report-more-details = Plus de détails
# Added 23 October 2026: what the report says beside a line's result (changed since the scan).
report-note-changed-since-scan = Modifié depuis l’analyse : il a été copié tel qu’il est maintenant.
report-note-no-longer-there = N’est plus là : il a été déplacé, renommé ou supprimé sur l’ancien ordinateur portable avant de pouvoir être envoyé.
report-note-time-not-kept = Date non conservée : ce disque ne peut pas enregistrer la date de sa dernière modification, il affiche donc la date de la copie.

# Evidence: how a copied item was checked.
evidence-read-back = Vérifié en le relisant depuis le disque.
evidence-memory = Vérifié depuis la mémoire de cet ordinateur portable, pas depuis le disque.
evidence-already-there = Déjà sur le nouvel ordinateur portable : le même fichier y était, il n’a donc pas été recopié.
evidence-recovered = Vérifié avant le redémarrage de l’ordinateur portable, puis de nouveau après.
evidence-new-laptop = Vérifié par le nouvel ordinateur portable.
evidence-memory-explain = La copie a été relue et correspondait, mais via la mémoire de cet ordinateur portable : ce qui a été lu venait peut-être de la mémoire plutôt que du disque. Un défaut du disque lui-même pourrait ne pas se voir ici, alors ouvrir le fichier une fois est une bonne vérification.
evidence-not-checked = Non vérifié, car il n’a pas été terminé.
# Machine-translated 16 October 2026.
why-memory-cannot-skip-cache = Ce disque n’a pas laissé PCTwin le lire directement, donc la vérification est passée par la mémoire de cet ordinateur portable. Certains disques réseau et clés USB fonctionnent ainsi.
why-memory-not-shown = Ce disque n’a pas pu montrer que la lecture de PCTwin l’avait vraiment atteint, donc la vérification compte comme faite depuis la mémoire de cet ordinateur portable.
why-memory-no-way = Le système de cet ordinateur portable ne peut pas lire ce disque directement, donc la vérification est passée par sa mémoire.
why-memory-stored-specially = Ce fichier est enregistré d’une manière particulière, par exemple compressé ou chiffré, donc le système le lit via la mémoire.
why-memory-too-small = Ce fichier est très petit, donc le disque le garde avec ses propres registres et le système le lit via la mémoire.

# Reasons: why an item did not finish. Each says what happened, then what to do.
reason-not-enough-space = Le nouvel ordinateur portable n’avait plus assez d’espace. Libérez de l’espace, puis lancez un nouveau transfert qui inclut cet élément.
reason-changed-during-move = Ce fichier changeait sans cesse sur l’ancien ordinateur portable pendant sa copie. Fermez toute appli qui l’utilise, puis lancez un nouveau transfert qui l’inclut.
reason-unreadable = L’ancien ordinateur portable n’a pas laissé PCTwin lire cet élément. Vérifiez qu’il s’ouvre sur l’ancien ordinateur, puis lancez un nouveau transfert qui l’inclut.
reason-not-verified = La copie ne correspondait pas à l’original, elle n’a donc pas été gardée. Lancez un nouveau transfert qui l’inclut. Si cela échoue encore, l’original est peut-être endommagé.
reason-refused-by-safety-check = Le contrôle de sécurité du nouvel ordinateur portable a refusé cet élément à cause de son nom, de son emplacement ou de sa taille. L’original est toujours sur l’ancien ordinateur, vous pouvez donc le copier autrement si besoin.
reason-name-not-allowed = Ce nom ne peut pas être utilisé sur le nouvel ordinateur portable, et aucun nouveau nom sûr n’était possible. Renommez-le sur l’ancien ordinateur, puis lancez un nouveau transfert qui l’inclut.
reason-install-failed = L’appli ne s’est pas installée. Installez-la depuis le site de son éditeur.
reason-no-equivalent = Cette appli n’existe pas pour le système du nouvel ordinateur portable. Cherchez une appli similaire qui fonctionne dessus.
reason-drive-locked = Le disque de destination est verrouillé. Déverrouillez-le (par exemple avec son mot de passe ou sa clé de récupération BitLocker), puis transférez-le à nouveau.
reason-sign-in = C’est en place, mais vous devez vous connecter. Ouvrez l’élément et connectez-vous.
reason-activation = L’appli est en place, mais elle a besoin de sa licence sur le nouvel ordinateur portable. Ouvrez-la et activez-la avec votre licence ou votre compte.
reason-permission = Le nouvel ordinateur portable doit d’abord donner son autorisation. Ouvrez l’élément et choisissez Autoriser quand c’est demandé.
reason-online-only = Ce fichier est stocké seulement en ligne, il n’était donc pas sur l’ancien ordinateur portable pour être copié. Téléchargez-le sur le nouvel ordinateur depuis l’appli qui le stocke en ligne.
reason-manual-install = Cette appli doit venir du site de son éditeur. Téléchargez-la et installez-la depuis ce site.
reason-restart = Le nouvel ordinateur portable doit redémarrer pour terminer. Redémarrez-le quand vous le souhaitez.
reason-finish-by-hand = PCTwin ne peut pas faire cette étape à votre place. Suivez les étapes indiquées pour elle dans ce rapport.
reason-parts-missing = Certains fichiers de ce dossier ne sont pas arrivés, il n’est donc pas complet sur le nouvel ordinateur. Ils sont listés dans ce rapport avec la raison. Envoyez-les à nouveau, ou copiez le dossier autrement.
reason-moved-file-missing = Ce réglage désigne un fichier ou un dossier qui n’est pas sur le nouvel ordinateur sous le nom que le transfert lui a donné, donc PCTwin a laissé le réglage tel quel. Ouvrez l’appli et choisissez à nouveau le fichier.

## Item results (every item ends with one of these)

status-copied = Copié
status-reinstalled = Réinstallé
status-converted = Converti
status-needs-action = Action nécessaire
status-skipped-by-user = Ignoré par vous
status-failed = Échec

## Screens: words the copy deck did not cover (same voice)

# $low and $high are whole minutes, a measured range. The result fills "About { $range } left".
progress-minutes = { $low } à { $high } minutes
# Machine-translated 16 October 2026.
progress-minutes-one =
    { NUMBER($minutes) ->
        [one] { $minutes } minute
        [many] { $minutes } de minutes
       *[other] { $minutes } minutes
    }
# $low and $high are numbers and $unit is already formatted, for example "MB/s". The result fills "{ $speed }, measured".
progress-speed-range = { $low } à { $high } { $unit }
progress-count-failed =
    { NUMBER($count) ->
        [one] { $count } élément n’a pas pu être copié
        [many] { $count } d’éléments n’ont pas pu être copiés
       *[other] { $count } éléments n’ont pas pu être copiés
    }
progress-queue-empty = Rien n’a encore commencé.
progress-approved-job = Transfert approuvé
progress-route-old = Ancien ordinateur portable
progress-route-new = Nouvel ordinateur portable
progress-panel-pausing = Mise en pause
outcome-metric-size = copiés et vérifiés
outcome-metric-action = demandent votre action
progress-percent-unknown = PCTwin calcule encore la taille de ce transfert.
mini-title-send-paused = Envoi en pause
cancel-ack-hint = Cochez la case ci-dessus pour activer le bouton d’arrêt.
progress-state-placing = Mise en place
progress-state-existing = Déjà présent
outcome-stopped-title = Votre transfert a été arrêté
outcome-stopped-body = Vous avez arrêté le transfert. Ce qui avait été copié et vérifié avant est gardé. Le reste n’a pas été copié.
outcome-broke-title = Votre transfert s’est arrêté tôt
outcome-broke-body = Un problème est survenu entre les ordinateurs portables, le transfert n’a donc pas pu continuer. Ce qui avait été copié et vérifié avant est gardé. Le reste n’a pas été copié.
# $done and $total are counts of items: what was copied and checked, and everything the person approved.
outcome-files-summary = Éléments copiés et vérifiés : { $done } sur { $total }.
outcome-files-memory =
    { NUMBER($count) ->
        [one] { $count } vérifié depuis la mémoire de cet ordinateur portable, pas depuis le disque.
        [many] { $count } vérifiés depuis la mémoire de cet ordinateur portable, pas depuis le disque.
       *[other] { $count } vérifiés depuis la mémoire de cet ordinateur portable, pas depuis le disque.
    }
report-show-more = Afficher plus
# $shown and $total are counts of items.
report-showing = Affichage : { $shown } sur { $total }

## Component kit

dialog-close = Fermer
toast-dismiss = Masquer
code-label = Code à six chiffres

## Shared Quick Send words

qs-eyebrow = Envoi rapide
qs-eyebrow-receiving = Réception sur votre ordinateur portable
qs-label-this = Ce portable
qs-label-other = L’autre portable
qs-pick-failed = Le sélecteur n’a pas pu s’ouvrir. Réessayez.
qs-handed =
    { NUMBER($count) ->
        [one] 1 fichier ou dossier a été confié à PCTwin pour l’envoi. Vérifiez la liste avant de continuer.
        [many] { $count } de fichiers ou dossiers ont été confiés à PCTwin pour l’envoi. Vérifiez la liste avant de continuer.
       *[other] { $count } fichiers ou dossiers ont été confiés à PCTwin pour l’envoi. Vérifiez la liste avant de continuer.
    }
qs-pick-too-many = C’est plus de fichiers qu’un seul envoi ne peut en emporter. Décochez quelques dossiers, ou envoyez-les en plusieurs fois.
qs-problem-lost-title = La connexion entre les ordinateurs a été perdue
qs-problem-failed-title = Les fichiers n’ont pas pu être envoyés cette fois
qs-problem-body = Ce qui a déjà été reçu et vérifié reste en place. Associez à nouveau les portables pour réessayer.
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = De { $from } vers { $to }
qs-route-sender = Envoi
qs-route-receiver = Réception · même Wi-Fi
qs-tally =
    { NUMBER($count) ->
        [one] { $count } élément sélectionné · { $size }
        [many] { $count } d’éléments sélectionnés · { $size }
       *[other] { $count } éléments sélectionnés · { $size }
    }
qs-tally-badge = Envoi à proximité

## Q01 Quick Send home

qs-home-title = Envoyez des fichiers à un portable proche
qs-home-body = Ou recevez-en d’un autre.
qs-send-title = Envoyer des fichiers
qs-send-body = Choisissez des fichiers ou des dossiers sur votre ordinateur portable.
qs-receive-title = Recevoir des fichiers
qs-receive-body = Laissez un ordinateur portable à proximité envoyer des fichiers au vôtre.
qs-home-action = Choisir des fichiers
qs-receiving = Prêt à recevoir. Les ordinateurs portables à proximité voient le vôtre sous le nom { $label }.
qs-receive-stop = Arrêter la réception
qs-no-nearby = Aucun ordinateur portable trouvé à proximité. Ouvrez PCTwin sur l’ordinateur qui envoie et gardez les deux sur le même Wi-Fi.

## Q02 Choose files and folders

qs-choose-title = Qu’envoyez-vous ?
qs-choose-body = Choisissez seulement les fichiers et dossiers que vous voulez partager.
qs-drop =
    .aria-label = Zone de dépôt des fichiers et dossiers
qs-drop-title = Déposez des fichiers ou des dossiers ici
qs-drop-body = Ou ajoutez-les avec Ajouter des fichiers ou Ajouter des dossiers.
qs-drop-active = Relâchez pour les ajouter.
qs-add-files = Ajouter des fichiers
qs-add-folders = Ajouter des dossiers
qs-remove =
    .aria-label = Retirer { $name }
# $kind is the file type, for example "PDF document" or "8 photos".
qs-file-detail = { $size } · { $kind }
qs-empty = Rien de choisi pour l’instant. Ajoutez des fichiers ou des dossiers à envoyer.
qs-online-only = Certains fichiers sont uniquement en ligne. Rendez-les d’abord disponibles sur votre ordinateur portable, puis envoyez-les.
qs-protected = PCTwin ne peut pas lire { $name }. Autorisez l’accès, ou retirez-le de la liste.
qs-allow = Autoriser l’accès
qs-choose-continue = Continuer

## Q12 Send progress

qs-sending-title = Envoi de vos fichiers
qs-sending-body = Les copies vont directement à l’ordinateur qui reçoit.
qs-sending-progress =
    .aria-label = Envoi des fichiers
qs-sending-current = Envoi en cours : { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = { $speed }, mesuré
qs-step-copying = Copie
qs-step-copying-body =
    { NUMBER($done) ->
        [one] { $done } fichier envoyé.
        [many] { $done } de fichiers envoyés.
       *[other] { $done } fichiers envoyés.
    }
qs-step-checking = Vérification
qs-step-checking-body = Chaque fichier est comparé à l’original.
qs-sending-paused = En pause. Les fichiers déjà reçus et vérifiés sont gardés.
qs-sending-disconnected = La connexion avec { $label } a été coupée. Les fichiers déjà vérifiés sont gardés. Nouvel essai automatique…
qs-sending-needs-action =
    { NUMBER($count) ->
        [one] { $count } élément demande votre action.
        [many] { $count } d’éléments demandent votre action.
       *[other] { $count } éléments demandent votre action.
    }
qs-pause = Mettre en pause
qs-resume = Reprendre
qs-compact = Afficher moins
qs-stop = Arrêter cet envoi
qs-sending-complete-title = L’envoi est terminé

## Q20 Stop this send? (and decline)

qs-stop-title = Arrêter cet envoi ?
qs-stop-body = Vérifiez ce qui est déjà arrivé avant d’arrêter.
qs-stop-arrived-title = Fichiers arrivés en entier
qs-stop-arrived-body = Restent dans le dossier choisi.
qs-stop-rest-title = Fichiers pas encore envoyés
qs-stop-rest-body = Rien de nouveau n’est envoyé après l’arrêt.
qs-stop-partial-title = Fichiers arrivés en partie
qs-stop-partial-body = Jamais enregistrés sous leur vrai nom, donc jamais confondus avec des copies complètes.
qs-stop-status-keep = Garder
qs-stop-status-stop = Arrêt
qs-stop-status-review = À vérifier
qs-stop-ack = Arrêter cet envoi
qs-stop-ack-hint = Les fichiers arrivés en entier ne sont pas supprimés.
qs-stop-keep = Continuer l’envoi

## F08 Your moves, kept on this laptop (history)

history-title = Votre historique des transferts
history-body = Rouvrez un rapport, ou reprenez un transfert enregistré.
history-row-move = { $from } → { $to }
history-status-finished = Terminé
history-status-steps-left =
    { NUMBER($count) ->
        [one] Terminé, { $count } étape pour vous
        [many] Terminé, { $count } d’étapes pour vous
       *[other] Terminé, { $count } étapes pour vous
    }
history-status-paused = En pause
history-status-stopped = Arrêté
history-open-report = Rapport
history-undo = Annuler
history-continue = Continuer
history-export = Exporter l’historique
history-exported = Historique enregistré sous { $path }.
history-export-failed = L’historique n’a pas pu être enregistré à cet endroit. Choisissez un autre dossier et réessayez.
history-note = Cet historique reste sur cet ordinateur portable. Rien n’est envoyé nulle part, sauf si vous l’exportez et le partagez vous-même.
history-empty = Votre premier transfert et son rapport apparaîtront ici.
history-empty-title = Aucun transfert enregistré
history-start = Commencer un transfert

## Quick Send: words the mockups do not have (added while building; same voice)

qs-soon-note = L’Envoi rapide n’est pas encore activé dans cette version. Rien n’a été envoyé ni reçu.
qs-tally-counting =
    { NUMBER($count) ->
        [one] { $count } élément sélectionné · taille en cours de calcul
        [many] { $count } d’éléments sélectionnés · taille en cours de calcul
       *[other] { $count } éléments sélectionnés · taille en cours de calcul
    }
qs-gone = { $name } n’est plus là. Il a peut-être été déplacé ou supprimé. Décochez-le, ou remettez-le en place et ajoutez-le à nouveau.
qs-choose-why-problems = Certains éléments cochés ne peuvent pas encore être envoyés. Décochez-les, ou corrigez-les comme indiqué, pour continuer.
qs-sending-card = Envoi des fichiers
qs-progress-of = sur { $total }
history-open-name = { $what } : { $title }

## R05 Annulation : ce que l’annulation fera sur chaque disque, et ce qu’est devenue chaque copie

undo-here-removes = L’annulation supprime les copies faites par PCTwin que vous n’avez pas modifiées. Tout ce que vous avez modifié est gardé.
undo-here-to-trash = L’annulation place dans la Corbeille les copies faites par PCTwin que vous n’avez pas modifiées. Tout ce que vous avez modifié est gardé, et rien n’est supprimé.
undo-here-keeps-fat = L’annulation gardera toutes les copies sur { $drive }. Ce disque utilise FAT ou exFAT, qui réutilise les numéros servant à distinguer les fichiers : PCTwin ne peut donc pas être sûr de savoir quels fichiers sont les siens. Vous pouvez les supprimer vous-même.
undo-here-keeps-other = L’annulation gardera toutes les copies sur { $drive }. Sur ce type de disque, PCTwin ne peut pas déplacer ni supprimer des fichiers en toute sécurité. Vous pouvez les supprimer vous-même.
undo-here-more = Plus de détails
undo-here-more-trash = Un Mac ne permet pas à PCTwin de savoir qu’aucune autre appli n’écrit encore dans un fichier : PCTwin n’en supprime donc jamais. Il place chaque copie dans la Corbeille du même disque. Vous pouvez la remettre en place depuis la Corbeille.
undo-here-more-fat = Sur les disques FAT et exFAT, un nouveau fichier peut recevoir le numéro d’un fichier supprimé. PCTwin n’agit que sur les fichiers dont il peut prouver qu’ils sont les siens et inchangés : il garde donc ceux-ci.
undo-done-trashed = Placé dans la Corbeille sous le nom « { $name } ». Vous pouvez le remettre en place depuis la Corbeille.
undo-kept-fat = Gardé. Ce disque (FAT ou exFAT) réutilise les numéros servant à distinguer les fichiers : PCTwin ne peut donc pas être sûr qu’il s’agit de sa copie. Si vous n’en avez pas besoin, vous pouvez la supprimer vous-même.
undo-kept-no-trash = Gardé. PCTwin n’a pas pu placer cette copie dans la Corbeille de son disque. Si vous n’en avez pas besoin, vous pouvez la supprimer vous-même.

## Quick Send (26 October 2026): pairing in Quick Send's words, where received files go, and the Done page

qs-pair-old-eyebrow = Envoi
qs-pair-old-body = Sur l’ordinateur qui reçoit, choisissez Recevoir des fichiers, puis saisissez ce code.
qs-pair-old-waiting = En attente de l’ordinateur qui reçoit…
qs-pair-old-label-hint = L’ordinateur qui reçoit trouvera le vôtre sous ce nom.
qs-pair-old-steps-title = Sur l’ordinateur qui reçoit
qs-pair-old-step-1 = Ouvrez PCTwin, choisissez Envoi rapide, puis Recevoir des fichiers.
qs-pair-old-left = L’ordinateur qui reçoit a arrêté l’association avant le choix d’un numéro, rien n’a donc été partagé. Voici un nouveau code.
qs-pair-network-hint = Choisissez le réseau de l’ordinateur qui reçoit.
qs-pair-choose-body = Regardez l’ordinateur qui reçoit. Choisissez le numéro qu’il affiche.
qs-pair-choose-renewed = De nouveaux numéros sont affichés. Choisissez celui de l’ordinateur qui reçoit.
qs-pair-choose-note = Si l’ordinateur qui reçoit n’affiche aucun numéro, ou si aucun ne correspond, choisissez Aucun ne correspond.
qs-pair-heads-up-firewall = Windows peut demander si PCTwin peut accepter des connexions. Choisissez Autoriser, pour que l’ordinateur qui reçoit puisse joindre le vôtre.
qs-pair-new-eyebrow = Réception
qs-pair-new-searching-title = Recherche de l’ordinateur qui envoie…
qs-pair-new-searching-body = Gardez PCTwin ouvert sur l’ordinateur qui envoie, sur le même Wi-Fi que le vôtre.
qs-pair-new-search-failed-title = Impossible de chercher l’ordinateur qui envoie
qs-pair-new-list-title = Quel ordinateur portable envoie ?
qs-pair-new-list-body = Choisissez le nom affiché sur l’écran de l’ordinateur qui envoie.
qs-pair-new-same-label = Un autre ordinateur portable à proximité porte ce nom. Vérifiez l’écran de l’ordinateur qui envoie.
qs-pair-new-none-title = Aucun ordinateur qui envoie trouvé
qs-pair-new-none-body = Vérifiez que PCTwin est ouvert sur l’ordinateur qui envoie et affiche un code, et que les deux ordinateurs sont sur le même Wi-Fi. Puis relancez la recherche.
qs-pair-new-code-hint = Lisez-le sur l’écran de { $label }. Il change chaque minute.
qs-pair-new-number-title = Choisissez ce numéro sur { $label }
qs-pair-new-number-body = Trois numéros sont affichés sur { $label }. Choisissez celui-ci là-bas.
qs-pair-new-number-waiting = En attente de { $label }…
qs-pairing-code-invalid = Ce code doit comporter six chiffres. Vérifiez l’écran de { $label } et saisissez-le à nouveau.
qs-pairing-code-wrong = Ce code ne correspond pas. Vérifiez l’écran de { $label } et réessayez.
qs-pairing-code-expired = Ce code a changé. Saisissez le nouveau code affiché sur l’écran de { $label }.
qs-pairing-busy = L’association avec { $label } ne peut pas commencer maintenant : il s’associe avec un autre ordinateur. Attendez un instant, puis réessayez.
qs-pairing-wait = Un nouveau code est en préparation sur { $label }. Attendez-le, puis saisissez le nouveau code.
qs-pairing-locked = L’association s’est arrêtée après trop d’essais erronés. Sur { $label }, choisissez Recommencer.
qs-pairing-unreachable = Votre ordinateur portable n’a pas pu joindre { $label }. Vérifiez que les deux sont sur le même Wi-Fi, puis réessayez.
qs-pairing-wrong-number = Ce numéro ne correspond pas à celui de l’ordinateur qui reçoit. Rien n’a été partagé. Saisissez le nouveau code sur l’ordinateur qui reçoit pour réessayer.
qs-pairing-no-match = Les numéros ne correspondaient pas sur { $label }. Rien n’a été partagé. Saisissez le nouveau code affiché sur son écran.
qs-pairing-timed-out = Aucun numéro n’a été choisi à temps. Réessayez avec le nouveau code de { $label }.
qs-pairing-connection-lost-new = La connexion avec { $label } a été coupée. Rien n’a été partagé.
qs-pairing-choose-network = Votre ordinateur portable est sur plusieurs réseaux. Choisissez celui de l’ordinateur qui reçoit :
qs-pairing-failed-new = L’association n’a pas fonctionné cette fois. Rien n’a été partagé. Saisissez le nouveau code affiché sur l’écran de { $label }.
qs-folder-line = Les fichiers seront enregistrés dans { $folder }.
qs-folder-change = Changer
qs-folder-title = Où enregistrer les fichiers ?
qs-folder-none = PCTwin ne peut pas utiliser le dossier des fichiers reçus. Choisissez un dossier pour les fichiers, puis connectez-vous.
qs-folder-choose = Choisir un dossier
qs-folder-failed = PCTwin ne peut pas enregistrer de fichiers dans ce dossier. Choisissez-en un autre.
qs-done-sent-title = Vos fichiers ont été envoyés
qs-done-here-title = Les fichiers ont été reçus
qs-done-partial-title = Certains fichiers ne sont pas arrivés
qs-done-stopped-title = Cet envoi a été arrêté
qs-done-empty-title = Rien n’a été envoyé
qs-done-arrived =
    { NUMBER($count) ->
        [one] { $count } fichier · { $size } arrivé et identique à l’original
        [many] { $count } de fichiers · { $size } arrivés et identiques aux originaux
       *[other] { $count } fichiers · { $size } arrivés et identiques aux originaux
    }
qs-done-arrived-count =
    { NUMBER($count) ->
        [one] { $count } fichier arrivé et identique à l’original
        [many] { $count } de fichiers arrivés et identiques aux originaux
       *[other] { $count } fichiers arrivés et identiques aux originaux
    }
qs-done-already =
    { NUMBER($count) ->
        [one] { $count } fichier était déjà là, identique, il n’a donc pas été renvoyé.
        [many] { $count } de fichiers étaient déjà là, identiques, ils n’ont donc pas été renvoyés.
       *[other] { $count } fichiers étaient déjà là, identiques, ils n’ont donc pas été renvoyés.
    }
qs-done-stopped-body = Les fichiers arrivés avant l’arrêt ont été vérifiés et restent là où ils ont été enregistrés. Rien d’autre n’a été envoyé.
qs-done-folder = Enregistrés dans { $folder }
qs-done-open-folder = Ouvrir le dossier
qs-done-dates-new =
    { NUMBER($count) ->
        [one] Ce fichier porte l’heure de son arrivée : il est en haut du dossier.
        [many] Ces { $count } de fichiers portent l’heure de leur arrivée : ils sont en haut du dossier.
       *[other] Ces { $count } fichiers portent l’heure de leur arrivée : ils sont en haut du dossier.
    }
qs-done-keep-dates = Garder les dates d’origine
qs-done-dates-kept =
    { NUMBER($count) ->
        [one] { $count } fichier a retrouvé sa date d’origine.
        [many] { $count } de fichiers ont retrouvé leur date d’origine.
       *[other] { $count } fichiers ont retrouvé leur date d’origine.
    }
qs-done-dates-failed = Les dates d’origine n’ont pas pu être remises. Les fichiers sont intacts et restent à leur place.
qs-done-open-failed = Le dossier n’a pas pu être ouvert. Il a peut-être été déplacé ou supprimé.
qs-done-not-opened = PCTwin n’a ouvert ni lancé aucun fichier reçu.
qs-done-problems-title = Fichiers à vérifier
qs-done-not-arrived = Il n’est pas arrivé.
qs-done-not-completed = Il n’a pas été envoyé avant l’arrêt.
qs-done-needs-action = Il est là, mais n’est pas terminé. Que faire est dans Plus de détails.
qs-done-mark-not-kept = Il est là, mais ce disque n’a pas pu le marquer comme téléchargé depuis un autre ordinateur portable. Ouvrez-le avec prudence.
qs-done-see-files = Voir les fichiers
qs-done-renamed = Enregistré sous { $name }, car un fichier portant ce nom était déjà là.
qs-done-identical = Un fichier identique était déjà là, il n’a donc pas été renvoyé.
qs-done-send-missing = Envoyer les fichiers manquants
qs-done-send-missing-body = Seulement les fichiers qui ne sont pas arrivés. Vous associez d’abord les ordinateurs à nouveau.
qs-done-done = Terminé
qs-stop-now = Arrêter l’envoi
qs-stop-finished = Cet envoi est déjà terminé.
qs-route-receiving = Réception
qs-receiving-title = Réception des fichiers
qs-receiving-body = Les copies viennent directement de { $label }.
qs-receiving-card = Réception des fichiers
qs-receiving-progress =
    .aria-label = Réception des fichiers
qs-receiving-current = Réception en cours : { $name }
qs-step-receiving-body =
    { NUMBER($done) ->
        [one] { $done } fichier arrivé.
        [many] { $done } de fichiers arrivés.
       *[other] { $done } fichiers arrivés.
    }
qs-tally-files =
    { NUMBER($count) ->
        [one] { $count } fichier · { $size }
        [many] { $count } de fichiers · { $size }
       *[other] { $count } fichiers · { $size }
    }
qs-receive-pause = Pause
qs-label-yours = Votre ordinateur portable
qs-label-receiving = Ordinateur qui reçoit
qs-label-sending = Ordinateur qui envoie
qs-peer-receiving = l’ordinateur qui reçoit
qs-peer-sending = l’ordinateur qui envoie
qs-pair-old-label-title = Nom de votre ordinateur portable
qs-pair-old-rename-label = Nom de votre ordinateur portable
qs-pair-old-step-2 = Si une liste apparaît, choisissez le nom de votre ordinateur portable.
qs-pair-old-locked-body = Après cinq codes erronés, l’association s’est arrêtée pour protéger votre ordinateur portable. Rien n’a été partagé. Si c’était vous, choisissez Recommencer pour obtenir un nouveau code.
qs-pair-new-needs-update = Il a une ancienne version de PCTwin. Mettez PCTwin à jour dessus, puis relancez la recherche.
qs-pairing-no-network = Votre ordinateur portable n’est connecté à aucun réseau Wi-Fi ou filaire. Connectez-le, puis réessayez.
qs-problem-room-title = Pas assez de place dans { $folder }
qs-problem-room-body = Ces fichiers ont besoin de { $needed }, et { $room } peuvent être utilisés là. Rien n’a été enregistré. Libérez de l’espace ou choisissez un autre dossier, puis réessayez.
qs-done-refused-title = L’ordinateur qui reçoit n’a pas pris ces fichiers
qs-done-refused-body = Il n’a peut-être pas assez de place. Rien n’a été envoyé.

## Écrans de la phase 2 (liste de tâches 2.8) : éléments communs

guard-state-not-started = Pas encore commencé
guard-state-preparing = Préparation
guard-state-awaiting-person = En attente de votre choix
guard-state-awaiting-system = En attente de l’autorisation de votre système
guard-state-awaiting-maker = En attente du créateur de l’appli ou du service en ligne
guard-state-working = En cours
guard-state-checking = Vérification de ce qui a été fait
guard-state-applying = Mise en place
guard-state-pause-requested = Mise en pause après l’étape en cours. Pas encore en pause.
guard-state-paused = En pause
guard-state-source-unavailable = L’ancien ordinateur portable ou le disque n’est pas connecté
guard-state-blocked = Il faut d’abord régler un problème
guard-state-partial = En partie terminé
guard-state-retrying = Nouvel essai
guard-state-reconciling = Vérification de ce qui a été terminé
guard-state-failed = Cela n’a pas marché
guard-state-complete = Terminé
guard-state-cancelled = Arrêté. Ce qui était déjà fait est gardé.
guard-refused-unavailable = PCTwin n’a pas pu vérifier cela pour le moment. Rien n’a été fait. Réessayez dans un instant.
guard-refused-wrong-job = Cela concernait un autre transfert. Rien n’a été fait.
guard-refused-old-session = Cet écran n’était plus à jour, donc rien n’a été fait. Voici ce qui est vrai maintenant.
guard-refused-plan-changed = Le plan a changé pendant que vous le regardiez, donc rien n’a été fait. Regardez-le à nouveau.
guard-refused-not-now = Cela ne peut pas se faire à cette étape. Rien n’a été fait.
guard-refused-not-shown = Ce choix n’est plus sur cet écran. Rien n’a été fait.
guard-progress-label = Avancement
guard-progress-mark = { $percent } % terminé
guard-progress-of = { $done } sur { $total }
guard-progress-unknown-total = { $done } pour l’instant. Le total n’est pas encore connu.
guard-progress-none = Rien n’a encore été mesuré.
guard-and-more =
    { $count ->
        [one] Et { $count } de plus, dans le rapport.
        [many] Et { $count } de plus, dans le rapport.
       *[other] Et { $count } de plus, dans le rapport.
    }
guard-not-known-yet = Pas encore connu
guard-free = { $free } libres
guard-free-unknown = Espace libre inconnu
guard-items-size = { $items } éléments, { $size }
guard-try-again = Réessayer
guard-pause = Pause
guard-resume = Reprendre
guard-check-again = Vérifier à nouveau
guard-strip-plan = Votre plan
guard-strip-report = Votre rapport
guard-strip-permission = Approbation
guard-strip-rescue = Secours
guard-strip-undo = Défaire
guard-strip-checking = Vérification

## X09 vérification après un redémarrage, et état indisponible

guard-checking-title = Vérification de ce qui a été terminé
guard-checking-body = PCTwin regarde ce qui était terminé avant sa fermeture. Vos choix reviennent dès qu’il le sait.
guard-unavailable-title = État indisponible
guard-unavailable-body = PCTwin ne peut pas dire pour le moment où en est ce transfert. Rien n’est affiché comme terminé tant qu’il ne le sait pas. Vérifiez à nouveau dans un instant.

## 2.9 Avant de changer quoi que ce soit

guard-changes-title = Ce que le transfert va changer
guard-changes-body = Voici tout ce que le transfert va changer sur cet ordinateur portable, et ce que l’annulation peut faire pour chaque chose. Rien n’a encore changé.
guard-changes-download = Téléchargement depuis internet
guard-changes-download-none = Aucun
guard-changes-permissions = Autorisation du système
guard-changes-one-prompt = Votre système demande une seule fois
guard-changes-no-prompt = Pas nécessaire
guard-changes-restore-point = Point de restauration Windows
guard-changes-restore-point-first = Demandé en premier
guard-changes-restore-point-none = Non utilisé
guard-changes-install =
    { $count ->
        [one] À installer : { $count } appli
        [many] À installer : { $count } d’applis
       *[other] À installer : { $count } applis
    }
guard-changes-merge =
    { $count ->
        [one] Ajouté à côté de ce qui est ici : { $count } élément
        [many] Ajoutés à côté de ce qui est ici : { $count } d’éléments
       *[other] Ajoutés à côté de ce qui est ici : { $count } éléments
    }
guard-changes-replace =
    { $count ->
        [one] À remplacer, en gardant l’ancienne version : { $count } élément
        [many] À remplacer, en gardant les anciennes versions : { $count } d’éléments
       *[other] À remplacer, en gardant les anciennes versions : { $count } éléments
    }
guard-changes-leave-alone =
    { $count ->
        [one] Laissé tel quel : { $count } élément
        [many] Laissés tels quels : { $count } d’éléments
       *[other] Laissés tels quels : { $count } éléments
    }
guard-changes-space = Place sur chaque disque au moment le plus chargé
guard-changes-why-prompt = Pourquoi votre système va demander
guard-changes-approve = Approuver ces changements
guard-undo-legend-can = PCTwin peut l’annuler.
guard-undo-legend-keeps = PCTwin le garde. L’annulation n’y touche pas.
guard-undo-legend-you = À vous d’agir. Seuls son service ou son créateur peuvent l’annuler.
guard-undo-can-reverse = PCTwin peut l’annuler
guard-undo-can-remove-app = PCTwin peut supprimer l’appli. Il ne touche pas aux fichiers que vous avez créés.
guard-undo-must-keep = Gardé : c’était déjà là
guard-undo-needs-you-service = À vous d’agir : seul le service en ligne peut l’annuler
guard-undo-needs-you-maker = À vous d’agir : les connexions et les licences dépendent du créateur
guard-drive-needs = Il faut { $needed }. { $free } libres.
guard-drive-needs-free-unknown = Il faut { $needed }. Le disque n’indique pas combien il reste de place.
guard-drive-fits = Ça rentre
guard-drive-short = Il manque { $short }
guard-drive-unknown = Inconnu
guard-permission-read-other-people = Pour lire les fichiers des autres personnes sur l’ancien ordinateur portable
guard-permission-make-accounts = Pour créer leurs comptes sur cet ordinateur portable
guard-permission-shared-folders = Pour mettre des choses dans des dossiers partagés que votre compte ne peut pas modifier
guard-permission-install-for-everyone = Pour installer des applis pour tous sur cet ordinateur portable
guard-permission-restore-point = Pour créer d’abord un point de restauration Windows

## X06 et H10 quelque chose a changé après l’approbation

guard-changed-title =
    { $count ->
        [one] { $count } chose a changé après votre approbation
        [many] { $count } de choses ont changé après votre approbation
       *[other] { $count } choses ont changé après votre approbation
    }
guard-changed-body = Seuls ces éléments sont à vérifier de nouveau. Tout le reste reste approuvé et continue.
guard-changed-changed-on-old-laptop = Cela a changé sur l’ancien ordinateur portable.
guard-changed-app-updated = L’appli s’est mise à jour toute seule.
guard-changed-destination-changed = L’endroit où cela va a changé.
guard-changed-new-scope = C’est nouveau : une autre personne ou un autre ordinateur portable a rejoint le transfert.
guard-changed-permission-ended = L’autorisation pour cela a pris fin.
guard-changed-leave-out = Laisser ceux-ci de côté
guard-changed-approve = Approuver à nouveau

## M31, M05 place sur le nouvel ordinateur portable

guard-space-title = Place sur le nouvel ordinateur portable
guard-space-title-short = Le nouvel ordinateur portable manque de place
guard-space-body = PCTwin a compté tout ce qui arrive sur chaque disque, y compris la place pour l’annulation. Rien de ce que vous avez choisi n’est abandonné pour que cela tienne.
guard-space-proposal = Ce que PCTwin propose pour ce qui ne tient pas
guard-keep-off-leave-in-cloud = Laissés en ligne, là où ils sont stockés
guard-keep-off-offload-drive = Mis sur votre autre disque
guard-keep-off-leave-on-old-laptop = Laissés sur l’ancien ordinateur portable. Gardez-le jusqu’à ce qu’ils soient transférés.
guard-offload-title = Votre autre disque
guard-offload-connected = Connecté
guard-offload-not-connected = Non connecté
guard-offload-approved = Vous avez choisi ce disque. Les fichiers qui y sont ne sont pas sur le nouvel ordinateur portable : connectez-le pour les ouvrir.
guard-offload-ask = PCTwin peut mettre certains éléments sur ce disque. Votre accord est nécessaire pour ce disque séparément.
guard-offload-use = Utiliser ce disque
guard-space-approve = Approuver ce plan
guard-safe-in = Stocké dans { $provider }
guard-safe-in-cloud = Stocké en ligne
guard-safe-in-detail =
    { $count ->
        [one] { $count } fichier est stocké en ligne uniquement. Il revient quand vous vous connectez sur le nouvel ordinateur portable.
        [many] { $count } de fichiers sont stockés en ligne uniquement. Ils reviennent quand vous vous connectez sur le nouvel ordinateur portable.
       *[other] { $count } fichiers sont stockés en ligne uniquement. Ils reviennent quand vous vous connectez sur le nouvel ordinateur portable.
    }
guard-your-cloud = votre stockage en ligne

## M09 garder une copie des fichiers uniquement en ligne

guard-local-title = Garder une copie de vos fichiers uniquement en ligne ?
guard-local-body =
    { $count ->
        [one] { $count } fichier dans { $provider } est stocké en ligne uniquement. Vous pouvez aussi en garder une copie sur le nouvel ordinateur portable.
        [many] { $count } de fichiers dans { $provider } sont stockés en ligne uniquement. Vous pouvez aussi en garder une copie sur le nouvel ordinateur portable.
       *[other] { $count } fichiers dans { $provider } sont stockés en ligne uniquement. Vous pouvez aussi en garder une copie sur le nouvel ordinateur portable.
    }
guard-local-download = À télécharger sur l’ancien ordinateur portable
guard-local-old-space = Place sur l’ancien ordinateur portable
guard-local-how = PCTwin les télécharge quelques-uns à la fois sur l’ancien ordinateur portable, les envoie, puis les remet en ligne uniquement sur celui-ci.
guard-local-paused-metered = En pause : cette connexion peut faire payer les données. Cela reprend tout seul sur une connexion gratuite, ou tout de suite si vous l’autorisez.
guard-local-paused-no-space = En pause : l’ancien ordinateur portable n’a plus de place pour les prochains fichiers. Les autres restent en sécurité dans { $provider }.
guard-local-done = Terminé. Les copies sont sur le nouvel ordinateur portable.
guard-local-keep = Garder une copie
guard-local-leave = Les laisser dans { $provider }
guard-local-allow-metered = Autoriser sur cette connexion

## M07 qui va où

guard-owners-title = Qui va où
guard-owners-body = Les affaires de chaque personne vont dans son propre compte sur cet ordinateur portable. Des noms identiques ne décident jamais seuls.
guard-owners-from = De { $laptop }
guard-owners-to-account = Vers le compte { $account } sur cet ordinateur portable
guard-owners-new-account = Vers un nouveau compte sur cet ordinateur portable
guard-owners-own-folder = Ses propres dossiers « Depuis { $laptop } », car les noms se chevauchent
guard-owners-usual-folders = Ses dossiers habituels
guard-owners-approve = Approuver

## X12 ce qui n’a pas été terminé

guard-report-title-partial = Certains éléments n’ont pas été transférés
guard-report-title-done = Tout dans cette partie a été transféré
guard-report-body-partial = Tout ce qui suit est encore sur l’ancien ordinateur portable. Réessayez chaque chose quand vous voulez. Ce qui a déjà été transféré reste.
guard-report-body-done = Il n’y a plus rien à réessayer.
guard-line-app = L’appli { $name }
guard-line-items =
    { $count ->
        [one] { $count } élément
        [many] { $count } d’éléments
       *[other] { $count } éléments
    }
guard-line-settings =
    { $count ->
        [one] { $count } réglage
        [many] { $count } de réglages
       *[other] { $count } réglages
    }
guard-line-why-failed = De { $laptop } : cela n’est pas terminé.
guard-line-why-laptop-not-connected = De { $laptop } : cet ordinateur portable n’est pas connecté. Connectez-le au même Wi-Fi et gardez-le allumé.
guard-line-why-waits-for-another = De { $laptop } : cela attend une autre étape qui n’est pas terminée. Réglez d’abord celle-là.
guard-line-why-not-started = De { $laptop } : le transfert s’est arrêté avant de commencer.

## M25, H10 l’autorisation du système, et le point de restauration

guard-permission-title-explain = PCTwin a besoin d’une autorisation de votre système
guard-permission-title-waiting-for-system = Répondez à la question de votre système
guard-permission-title-denied = Vous n’avez pas donné votre accord
guard-permission-title-closed-without-answer = La question a été fermée sans réponse
guard-permission-title-given = Approuvé pour ce transfert
guard-permission-title-ended-by-sleep = L’autorisation a pris fin quand l’ordinateur portable s’est mis en veille
guard-permission-title-ended-other-session = L’autorisation a pris fin quand quelqu’un d’autre s’est connecté
guard-permission-body-explain = Votre système demande une seule fois pour tout le transfert. Seules les étapes listées ci-dessous s’en servent, et elle prend fin avec le transfert.
guard-permission-body-waiting-for-system = La fenêtre de votre système est ouverte. Tapez-y votre mot de passe s’il le demande.
guard-permission-body-denied = Seules les étapes qui en avaient besoin sont laissées de côté. Tout le reste continue.
guard-permission-body-closed-without-answer = Fermer la fenêtre ne vaut pas accord, donc rien de ce qui en a besoin n’a été fait. Demandez à nouveau, ou continuez sans elle.
guard-permission-body-given = PCTwin ne s’en sert que pour les étapes listées ci-dessous, et elle prend fin avec ce transfert.
guard-permission-body-ended-by-sleep = Rien de ce qui en a besoin n’a été fait après cela. Demandez à nouveau pour continuer.
guard-permission-body-ended-other-session = Rien de ce qui en a besoin n’a été fait après cela. Demandez à nouveau quand vous serez de retour sur votre compte.
guard-permission-never-password = PCTwin ne demande jamais votre mot de passe et ne le voit jamais. Seul votre système le voit.
guard-permission-not-now = Continuer sans
guard-permission-ask = Continuer vers votre système
guard-permission-ask-again = Demander à nouveau
guard-restore-title = Point de restauration Windows
guard-restore-made = Créé à { $time }.
guard-restore-earlier-exists = Windows en a déjà un de { $time }, donc il n’en a pas créé d’autre.
guard-restore-not-chosen = Pas créé : vous avez choisi de ne pas en faire.
guard-restore-not-made-not-windows = Pas créé : les points de restauration n’existent que sous Windows.
guard-restore-not-made-protection-off = Pas créé : System Protection est désactivée pour ce disque. Vous pouvez l’activer dans System Properties.
guard-restore-not-made-needs-admin = Pas créé : il faut l’autorisation de votre système.
guard-restore-not-made-unconfirmed = Pas créé : Windows a dit oui, mais on ne trouve aucun point de restauration.
guard-restore-not-made-failed = Pas créé : Windows n’a pas pu en créer un.

## T01 à T04 secours depuis un ordinateur portable qui ne démarre pas

guard-rescue-title-intro = Copier les fichiers d’un ordinateur portable qui ne démarre plus
guard-rescue-title-pick-drive = Choisissez le disque à sauver
guard-rescue-title-locked = Ce disque est verrouillé
guard-rescue-title-no-system = Ce n’est pas le disque d’un ordinateur portable
guard-rescue-title-salvage = Voici ce qui a pu être lu
guard-rescue-body-intro = Sortez le disque de l’ancien ordinateur portable et branchez-le à celui-ci avec un adaptateur ou un boîtier pour disque.
guard-rescue-body-pick-drive = Choisissez le disque de l’ancien ordinateur portable. Le disque vers lequel vous copiez ne peut pas être celui que vous sauvez.
guard-rescue-body-locked = Déverrouillez-le d’abord dans votre système. Puis vérifiez à nouveau.
guard-rescue-body-no-system = Ce disque n’a ni système d’ordinateur portable ni dossiers personnels. Si c’est une simple clé USB, utilisez-la plutôt sur le nouvel ordinateur portable.
guard-rescue-body-salvage = Regardez la liste, puis utilisez ce qui a pu être lu.
guard-rescue-can = Ce qu’un secours peut faire
guard-rescue-can-body = Lire vos fichiers, ainsi que les personnes et les réglages trouvés sur le disque, depuis un disque qui fonctionne encore.
guard-rescue-cannot = Ce qu’il ne peut pas faire
guard-rescue-cannot-body = Réparer le disque, lire les parties abîmées, ou ouvrir un disque dont vous n’avez pas la clé.
guard-rescue-read-only-title = Le disque est seulement lu
guard-rescue-read-only-promise = PCTwin ne fait que le lire. Rien dessus n’est changé, réparé ni formaté.
guard-rescue-system-read-only = Votre système l’a ouvert en lecture seule, donc rien dessus ne peut changer.
guard-rescue-pctwin-reads-only = PCTwin l’a seulement lu. Votre système l’avait ouvert en modification, donc d’autres applis pouvaient encore le changer.
guard-rescue-refused-target = Ce disque est l’endroit où les choses sont mises, il ne peut donc pas aussi être sauvé. Choisissez un autre disque.
guard-rescue-drive-detail = { $size }, { $fs }
guard-rescue-drive-is-target = Les choses du transfert vont sur ce disque.
guard-rescue-drive-cannot = Ne peut pas être sauvé
guard-rescue-choose = Sauver ce disque
guard-rescue-bit-locker-step-1 = Ouvrez le disque dans File Explorer.
guard-rescue-bit-locker-step-2 = Tapez sa clé de récupération quand Windows la demande. Elle est souvent enregistrée dans le compte Microsoft du propriétaire, sur aka.ms/myrecoverykey.
guard-rescue-bit-locker-step-3 = Revenez ici et choisissez Vérifier à nouveau.
guard-rescue-file-vault-step-1 = Déverrouillez-le sur un Mac avec le mot de passe de session du propriétaire ou sa clé de récupération.
guard-rescue-file-vault-step-2 = Revenez ici et choisissez Vérifier à nouveau.
guard-rescue-apple-chip-step-1 = Ce Mac garde son disque verrouillé pour lui seul, donc sortir le disque ne marche pas.
guard-rescue-apple-chip-step-2 = Si le Mac démarre encore dans Recovery, utilisez Share Disk avec un câble. S’il ne démarre pas du tout, seul un centre de réparation Apple peut aider.
guard-rescue-never-key = PCTwin ne demande jamais la clé et ne la garde jamais.
guard-rescue-readable =
    { $count ->
        [one] { $n } élément peut être lu.
        [many] { $n } d’éléments peuvent être lus.
       *[other] { $n } éléments peuvent être lus.
    }
guard-rescue-unreadable =
    { $count ->
        [one] { $count } élément n’a pas pu être lu
        [many] { $count } d’éléments n’ont pas pu être lus
       *[other] { $count } éléments n’ont pas pu être lus
    }
guard-rescue-look = Chercher le disque
guard-rescue-check-again = Vérifier à nouveau
guard-rescue-use = Utiliser ce qui a pu être lu

## R05, R06 annulation

guard-undo-preview-title = Ce que l’annulation va faire
guard-undo-preview-body = PCTwin vient de regarder chaque changement. Tout ce que vous avez modifié après le transfert est gardé. Rien ne se passe tant que vous ne choisissez pas Annuler.
guard-undo-preview-which = Ceci annule le transfert terminé le { DATETIME($when, year: "numeric", month: "long", day: "numeric", hour: "numeric", minute: "2-digit") }. Vos autres transferts ne changent pas.
guard-undo-preview-which-earlier = Ceci annule le transfert fait avec une version précédente de PCTwin. Vos autres transferts ne changent pas.
guard-undo-preview-admin = Pour supprimer des applis, cet ordinateur demandera l’accord d’un administrateur.
guard-undo-plan-put-back = Remis comme avant
guard-undo-plan-remove-app = PCTwin supprime l’appli. PCTwin ne touche pas aux fichiers que vous avez créés.
guard-undo-plan-keep-changed = Gardé : vous l’avez modifié
guard-undo-plan-elsewhere = À vous d’agir : fait ailleurs
guard-undo-plan-leave-alone = Laissé tel quel : c’était déjà là
guard-undo-keep = Tout garder tel quel
guard-undo-run = Défaire
guard-undo-running-title = Annulation des changements
guard-undo-done-title = Ce que l’annulation a fait
guard-undo-result-restored = Restauré
guard-undo-result-removed = Supprimé
guard-undo-result-as-before = Déjà comme avant
guard-undo-result-kept-changed = Gardé car vous l’avez modifié
guard-undo-result-kept-other = Gardé, voir Plus de détails
guard-undo-result-kept-at = Gardé sous le nom { $name }
guard-undo-result-done-elsewhere = Fait ailleurs : seul son service ou son créateur peut l’annuler
guard-undo-result-not-done-yet = Pas encore fait. L’annulation réessaiera la prochaine fois.

## The full move: apps, settings and app data (28 October 2026)

fullmove-plan-title = Applications, réglages et données d’applications
fullmove-plan-body = PCTwin a examiné les applications, les réglages et les données d’applications de votre ancien ordinateur. Voici ce qu’il fera sur cet ordinateur après vos fichiers. Rien ne change avant que vous lanciez le transfert.
fullmove-group-install = Applications que PCTwin installera : { $count }
fullmove-group-settings = Réglages d’applications que PCTwin copiera : { $count }
fullmove-group-needs = Ce que vous ferez vous-même : { $count }
fullmove-group-here = Applications déjà sur cet ordinateur : { $count }
fullmove-settings-count = Réglages : { $count }.
fullmove-settings-merge = Ils sont ajoutés. Rien n’est remplacé sur cet ordinateur.
fullmove-settings-replace = Certains réglages de cet ordinateur sont remplacés. L’annulation peut les remettre.
fullmove-app-install = PCTwin l’installe depuis son créateur et la vérifie.
fullmove-app-here = Elle est déjà sur cet ordinateur, donc PCTwin la laisse telle quelle.
fullmove-step-copy = Copié, avec annulation possible.
fullmove-step-export-first = Sur votre ancien ordinateur, utilisez l’exportation de l’application. Puis importez-la dans l’application sur cet ordinateur.
fullmove-step-sign-in = Reconnectez-vous sur cet ordinateur.
fullmove-step-relink = Configurez-la à nouveau sur cet ordinateur, ou reliez-la depuis l’application.
fullmove-step-not-yet = PCTwin ne transfère pas encore les données de cette application. Elles restent en sécurité sur votre ancien ordinateur.
fullmove-step-look-at-it = PCTwin n’a pas pu savoir ce que c’est. Regardez-le sur votre ancien ordinateur.
fullmove-group-update =
    { $count ->
        [one] Une version plus ancienne est sur ce portable
        [many] Des versions plus anciennes sont sur ce portable
       *[other] Des versions plus anciennes sont sur ce portable
    }
fullmove-update-it = Mettre à jour { $name }
fullmove-update-hint = PCTwin installe la version plus récente. Vos réglages restent.
fullmove-special-not-yet = Données d’applications comme les e-mails, les navigateurs et les jeux
fullmove-left-by-person = Applications, réglages et données d'applications : laissés sur votre ancien portable, comme vous l'avez choisi
fullmove-special-finish = PCTwin n’a pas transféré ceci. C’est toujours en sécurité sur votre ancien ordinateur.
fullmove-more = Et { $count } de plus.
fullmove-not-moved-title = Réglages que PCTwin laisse sur l’ancien ordinateur
fullmove-not-moved-private = Réglages privés et de sécurité, et connexions, jamais copiés : { $count }
fullmove-not-moved-could-run = Réglages qui lanceraient ou ouvriraient quelque chose, gardés par sécurité : { $count }
fullmove-not-moved-old-laptop-only = Réglages propres à l’ancien ordinateur : { $count }
fullmove-not-moved-system-own = Réglages du système que PCTwin ne transfère pas encore : { $count }
fullmove-not-moved-not-read = Réglages trouvés mais non lus : { $count }
fullmove-special-not-moved = Fichiers d’applications laissés de côté, comme les caches : { $count }
fullmove-prompt = Cet ordinateur demandera une fois l’autorisation d’un administrateur.
fullmove-prompt-restore = Cet ordinateur demandera une fois l’autorisation d’un administrateur et créera un point de restauration avant le premier changement.
fullmove-rebuilding-title = Préparation de vos applications et réglages
fullmove-rebuilding-body = Vos fichiers sont en place. PCTwin prépare maintenant le reste. Gardez cet ordinateur allumé.
fullmove-rebuilding-permission = Demande d’autorisation à cet ordinateur
fullmove-rebuilding-apps = Applications : { $done } sur { $total }
fullmove-rebuilding-settings = Réglages : { $done } sur { $total }
fullmove-outcome-title = Applications, réglages et données d’applications
fullmove-outcome-undo = Annuler les réglages d’apps
fullmove-outcome-stopped = PCTwin a arrêté de modifier les réglages en cours de route, car il n’a pas pu noter sa modification suivante. Rien n’a changé après ce point.
fullmove-restore-made = Un point de restauration a été créé avant les changements.
fullmove-restore-earlier-exists = Aucun nouveau point de restauration n’a été créé, car cet ordinateur en avait déjà un d’aujourd’hui.
fullmove-restore-not-made = Aucun point de restauration n’a été créé. L’annulation de PCTwin fonctionne toujours.
fullmove-restore-not-windows = Cet ordinateur n’a pas de points de restauration. L’annulation de PCTwin fonctionne toujours.
guard-undo-remove-yourself = Supprimez-la vous-même si vous voulez
guard-undo-plan-remove-yourself = Vous pouvez la supprimer dans les paramètres de votre ordinateur
guard-undo-result-remove-yourself = À supprimer vous-même dans les paramètres de votre ordinateur
guard-undo-plan-keep-other-version = Gardée : elle a changé depuis le transfert
guard-undo-plan-already-gone = Déjà supprimé
guard-undo-result-kept-other-version = Gardée, car elle a changé depuis le transfert
guard-undo-result-removed-restart = Supprimé. Redémarrez votre ordinateur pour terminer.
guard-undo-result-not-removed = Non supprimé. Vous pouvez le supprimer dans les paramètres de votre ordinateur.
guard-undo-close = Terminé
move-wait-appdata-title = Examen des données de vos applications
move-wait-appdata-body = Vos fichiers sont trouvés. PCTwin examine encore les données que gardent vos applications, comme les e-mails et les navigateurs. Cela prend dix minutes au plus. Rien n’est modifié sur cet ordinateur.
fullmove-special-not-checked = Données d’applications que PCTwin n’a pas pu vérifier entièrement
fullmove-group-appdata = Données d’applications que PCTwin copiera : { $count }
fullmove-group-secrets = Fichiers secrets que vous pouvez choisir d’apporter : { $count }
fullmove-secrets-body = Ils contiennent des mots de passe ou des clés. PCTwin n’en apporte un que si vous le cochez vous-même. Non coché, il reste sur votre ancien ordinateur.
fullmove-secret-bring = Apporter { $name }
fullmove-secret-hint = Vous seul pourrez l’ouvrir sur cet ordinateur. L’annulation peut le retirer.
fullmove-appdata-title = Copie des données de vos applications
fullmove-appdata-progress = Copie des données d’applications : { $done } sur { $total }
fullmove-appdata-waiting = En attente de l’ancien ordinateur : fermez { $apps } là-bas.
fullmove-close-title = Fermez ces applications pour apporter leurs données
fullmove-close-body = Ces applications sont encore ouvertes sur cet ordinateur, PCTwin ne peut donc pas copier leurs données sans risque. Fermez-les, puis choisissez Réessayer. Si vous continuez sans elles, leurs données restent ici.
fullmove-close-again = Réessayer
fullmove-close-without = Continuer sans elles
fullmove-extras-refused = Les réglages et données d’applications de l’ancien ordinateur n’ont pas pu être lus ici, donc rien n’a été transféré. Ils sont toujours sur votre ancien ordinateur.
fullmove-app-from = PCTwin installe le paquet winget { $id } de son créateur, après avoir vérifié que c’est cette application et le même créateur.
fullmove-here-older = Une version plus ancienne ({ $version }) est sur cet ordinateur. PCTwin la laisse telle quelle.
fullmove-step-sites = Les connexions enregistrées aux sites web restent liées à l’ancien ordinateur. Reconnectez-vous à ces sites sur cet ordinateur.
fullmove-app-rosetta = Conçue pour les anciens Mac Intel. Sur ce Mac, elle fonctionne grâce à Rosetta, qu’Apple va arrêter : elle pourrait cesser de fonctionner après une future mise à jour de macOS. Cherchez une version plus récente chez son éditeur.
fullmove-links = Raccourcis vers d’autres endroits parmi vos fichiers, que PCTwin ne suit pas et ne transfère pas : { $count }
fullmove-note-sign-in = Ouvrez-la et connectez-vous à votre compte.
fullmove-note-licence-key = Ouvrez-la et saisissez votre clé de licence, ou connectez-vous au compte qui détient votre licence.
fullmove-note-registration = Votre ancien ordinateur l’indiquait comme enregistrée au nom de { $who }.
fullmove-note-product-id = Son identifiant de produit sur votre ancien ordinateur était { $id }.
fullmove-note-extras = D’autres programmes sont arrivés avec elle : { $apps }. Supprimez-les dans les paramètres de votre ordinateur si vous n’en voulez pas.
fullmove-note-rosetta = Conçue pour les Mac Intel, elle a besoin de Rosetta. Si ce Mac propose d’installer Rosetta quand vous ouvrez l’app, choisissez Installer.

fullmove-note-came-from-maker = La copie de son programme d’installation sur votre ancien ordinateur portable venait du site web de l’éditeur.
fullmove-note-came-from-elsewhere = La copie de son programme d’installation sur votre ancien ordinateur portable ne venait pas du site web de l’éditeur. PCTwin ne récupère les applis qu’auprès de l’éditeur.
fullmove-data-lost = La connexion a été perdue avant l’arrivée de ces données. Elles sont toujours en sécurité sur votre ancien ordinateur. Lancez un nouveau transfert pour les apporter.
fullmove-data-did-not-arrive = Ces données ne sont pas arrivées. Elles sont toujours en sécurité sur votre ancien ordinateur. Lancez un nouveau transfert pour les apporter.
fullmove-data-app-open = L’application était ouverte sur votre ancien ordinateur, donc ses données n’ont pas été copiées. Fermez-la là-bas, puis lancez un nouveau transfert pour les apporter.
fullmove-data-changing = Ces données changeaient sur votre ancien ordinateur pendant que PCTwin les lisait. Fermez l’application là-bas, puis lancez un nouveau transfert pour les apporter.
fullmove-data-unreadable = PCTwin n’a pas pu lire ces données sur votre ancien ordinateur. Elles y sont toujours.
fullmove-data-not-read = Le transfert s’est arrêté avant la lecture de ces données. Elles sont toujours en sécurité sur votre ancien ordinateur.
fullmove-data-not-whole = Ces données ne sont pas toutes arrivées, donc PCTwin n’en a mis aucune en place. Elles sont toujours en sécurité sur votre ancien ordinateur.
fullmove-data-not-landed = Ces données sont arrivées, mais PCTwin n’a pas pu les mettre en place sur cet ordinateur. Elles sont toujours en sécurité sur votre ancien ordinateur.
fullmove-data-beside = L’application avait déjà son propre dossier sur cet ordinateur, donc PCTwin a placé la copie de l’ancien ordinateur à côté, dans { $folder }. L’application utilise toujours son propre dossier. Pour utiliser l’ancienne copie, fermez l’application et échangez les deux dossiers.
fullmove-data-placed = Les données sont dans { $folder }, mais l’application a besoin d’une étape avant de pouvoir les utiliser. Ouvrez l’application et vérifiez-les.
fullmove-outcome-here-title = Applications déjà sur cet ordinateur : { $count }
fullmove-outcome-here = Déjà sur cet ordinateur, laissée telle quelle.
fullmove-outcome-here-older = Une version plus ancienne ({ $version }) est sur cet ordinateur. PCTwin l’a laissée telle quelle. Mettez-la à jour depuis son créateur si vous voulez la plus récente.
fullmove-outcome-here-badge = Laissée telle quelle
fullmove-outcome-fresh = Les applications installées par PCTwin repartent de zéro. Ouvrez chacune et connectez-vous si elle le demande.
progress-may-sleep = Cet ordinateur risque de se mettre en veille pendant le transfert. Gardez-le branché et ouvert.

## The old laptop chooses what moves (decided 28 October 2026)
choose-title = Que voulez-vous déplacer ?
choose-body = Choisissez ici, sur le portable où sont vos affaires. Votre nouveau portable vous montrera ensuite ce qui arrive et vous demandera de l'accepter.
choose-mode = Quoi déplacer
choose-everything = Tout
choose-except = Tout sauf
choose-only = Seulement ce que je choisis
choose-except-hint = Décochez ce qu'il faut laisser.
choose-open = Ouvrir { $name }
choose-close = Fermer { $name }
choose-total =
    { $files ->
        [one] 1 fichier
        [many] { $files } fichiers
       *[other] { $files } fichiers
    } · Choisi : { $size }
choose-only-hint = Cochez ce qu'il faut emporter.
choose-apps = Applications, réglages et données d'applications
choose-apps-count =
    { $count ->
        [one] 1 application, avec ses réglages
        [many] { $count } applications, avec leurs réglages
       *[other] { $count } applications, avec leurs réglages
    }
choose-group-size =
    { $files ->
        [one] 1 fichier
        [many] { $files } fichiers
       *[other] { $files } fichiers
    } · { $size }
choose-left-title = Laissé de toute façon
choose-left-badge = Laissé
choose-left-caches = Caches et fichiers temporaires
choose-left-chosen-by-user = Ce que vous avez choisi de laisser
choose-left-work-managed = Les fichiers de votre travail ou de votre école
choose-left-device-bound = Ce qui est lié à ce portable, comme les clés d'accès
choose-left-unreadable = Les fichiers que PCTwin n'a pas pu lire
choose-left-cloud-only = Les fichiers gardés uniquement en ligne
choose-left-system = Des parties de Windows lui-même
choose-left-too-many-files = Des dossiers trop grands pour un seul déplacement
choose-left-rebuildable = Ce que votre nouveau portable peut recréer, comme les paquets téléchargés
choose-nothing = Choisissez au moins une chose à déplacer.
choose-continue = Continuer

## The new laptop accepts what the old laptop chose (decided 28 October 2026)
accept-title = Accepter le transfert depuis { $from } ?
accept-body = Cela a été choisi sur votre ancien portable. Acceptez, et PCTwin le déplace ici.
accept-files =
    { $count ->
        [one] 1 fichier
        [many] { $count } fichiers
       *[other] { $count } fichiers
    }
accept-apps =
    { $count ->
        [one] 1 application à installer
        [many] { $count } applications à installer
       *[other] { $count } applications à installer
    }
accept-settings = Réglages et données d'applications
accept-fits = Cela tient sur ce portable.
accept-needs = Quelque chose demande votre attention d'abord. Voyez les détails.
accept-not-now = Pas maintenant
accept-details = Voir les détails
accept-accept = Accepter

# 2.5 special categories
special-category-email = E-mail
special-category-browser = Navigateurs
special-category-game = Jeux
special-category-ai-tool = Outils d'IA et éditeurs de code
special-category-secret = Clés et mots de passe
special-category-other = Autres données d'applications
special-note-search-rebuilds = Votre application de messagerie refait sa recherche à la première ouverture. La recherche peut être lente pendant un moment.
special-note-server-resync = Vos e-mails reviennent du serveur de messagerie quand vous vous connectez. PCTwin garde l'ancienne copie seulement au cas où quelque chose ne serait jamais arrivé au serveur.
special-note-own-sync = Le plus simple : activez la synchronisation de ce navigateur sur votre nouvel ordinateur. Elle ramène vos favoris, votre historique, vos onglets ouverts et vos extensions.
special-note-passwords-guided = Les mots de passe enregistrés ne passent que par l'export et l'import du navigateur lui-même, ou par sa synchronisation. Le fichier exporté n'est pas protégé : PCTwin le garde privé et le supprime après l'import.
special-note-passkeys-guided = Les clés d'accès enregistrées dans un compte reviennent quand vous vous y connectez. Celles gardées seulement sur cet ordinateur doivent être recréées sur chaque site. Faites-le avant d'effacer l'ancien ordinateur.
special-note-cloud-saves-may-differ = Le lanceur du jeu peut aussi garder vos sauvegardes en ligne. S'il vous demande lesquelles garder, regardez d'abord les dates.
special-note-models-checked-not-run = Les modèles d'IA sont vérifiés fichier par fichier à leur arrivée. PCTwin ne les lance jamais.
special-note-tool-settings-review = Ces réglages peuvent lancer des programmes ou accéder à Internet. Vérifiez-les avant de les utiliser sur le nouvel ordinateur.
special-note-secret-tick-each = Ceci ne bouge que si vous le cochez. Il n'apparaît jamais dans un rapport.
browser-file-bookmarks = Favoris.html
browser-file-lists = Pages ouvertes et extensions.html
browser-file-bookmarks-title = Favoris
browser-file-add-ons = Extensions
browser-file-open-pages = Pages ouvertes
special-note-open-archive-in-mail-app = Ouvrez cette archive de courrier dans votre app de messagerie sur le nouveau portable. Dans Outlook : Fichier, Ouvrir, Fichier de données Outlook.
special-note-local-only-folders-export = Les dossiers marqués « Cet ordinateur uniquement » ne sont pas sur le serveur de messagerie. Avant d'effacer l'ancien portable, exportez-les depuis votre app de messagerie vers un fichier d'archive, puis déplacez ce fichier.
special-note-bookmarks-file = Vos favoris sont aussi enregistrés dans un fichier que tout navigateur peut importer. Si le nouvel ordinateur a un autre navigateur, choisissez son option pour importer des favoris depuis un fichier HTML, puis choisissez ce fichier. Il se trouve dans le dossier « PCTwin brought over ».
special-note-history-own-import = Les navigateurs différents ne partagent pas de format commun pour l'historique. Pour passer l'historique à un autre navigateur, utilisez l'import de ce navigateur pendant que les deux navigateurs sont sur le même ordinateur, ou activez la synchronisation du navigateur.
special-note-pages-and-add-ons-listed = La liste de vos extensions et des pages que vous aviez ouvertes est enregistrée à côté du fichier de favoris. Elle n'est que pour vous. PCTwin n'ouvre aucune de ces pages.
special-note-open-pages-by-sync = PCTwin ne peut pas lister les pages que vous aviez ouvertes dans ce navigateur. Pour les rouvrir, activez la synchronisation du navigateur, ou son réglage qui ouvre les pages de la dernière fois.
forgotten-title = Ce que l'on oublie souvent
forgotten-kind-fonts = Vos propres polices
forgotten-kind-mail-signatures = Signatures d'e-mail
forgotten-kind-dictionaries = Vos propres mots du dictionnaire
forgotten-kind-office-templates = Modèles Office
forgotten-kind-mail-archives = E-mails gardés sur cet ordinateur
forgotten-kind-game-saves = Sauvegardes de jeux
forgotten-kind-game-library = Jeux installés par un lanceur de jeux
forgotten-moving = Ceci part avec vos autres affaires.
forgotten-not-moving = Ceci ne fait pas partie du transfert. Vous pouvez le copier vous-même.
forgotten-step-install = Après le transfert, ouvrez chaque police sur le nouvel ordinateur et choisissez Installer.
forgotten-step-open-in-mail-app = Ouvrez cette archive de courrier dans votre app de messagerie sur le nouveau portable. Dans Outlook : Fichier, Ouvrir, Fichier de données Outlook.
forgotten-step-locate-verify = Installez le lanceur de jeux sur le nouvel ordinateur et connectez-vous. Il peut réinstaller vos jeux. Si vous copiez ce dossier, utilisez l'option du lanceur pour retrouver les jeux installés et laissez-le vérifier les fichiers.
forgotten-not-whole = PCTwin a arrêté de compter ici : il peut y en avoir plus.
forgotten-printers = Imprimantes à ajouter de nouveau
forgotten-printer-step = Ajoutez de nouveau cette imprimante sur votre nouvel ordinateur.
forgotten-printer-network = Partagée depuis un autre ordinateur
forgotten-not-checked = PCTwin n'a pas pu regarder ici.
forgotten-files =
    { $count ->
        [one] 1 fichier
        [many] { $count } fichiers
       *[other] { $count } fichiers
    }
forgotten-online-only =
    { $count ->
        [one] 1 fichier gardé seulement en ligne, non téléchargé
        [many] { $count } fichiers gardés seulement en ligne, non téléchargés
       *[other] { $count } fichiers gardés seulement en ligne, non téléchargés
    }

# 2.7 finishing a move
doctor-not-connected = Cet ordinateur portable n’est connecté à aucun réseau.
doctor-not-connected-fix = Connectez-vous au Wi-Fi ou branchez un câble réseau, puis réessayez.
doctor-mac-local-network-denied = Ce Mac n’autorise peut-être pas PCTwin à communiquer avec les appareils de votre réseau.
doctor-mac-local-network-denied-fix = Ouvrez Réglages Système, puis Confidentialité et sécurité, puis Réseau local, et activez PCTwin.
doctor-discovery-blocked = Quelque chose sur cet ordinateur, comme un pare-feu ou une app de sécurité, empêche peut-être les ordinateurs de se trouver.
doctor-discovery-blocked-fix = Autorisez PCTwin dans votre pare-feu ou votre app de sécurité, ou mettez-le en pause pendant le transfert.
doctor-windows-public-network = Windows traite le réseau { $name } comme public, ce qui bloque les autres appareils.
doctor-windows-public-network-fix = Ouvrez Paramètres, puis Réseau et Internet, choisissez { $name } et réglez-le sur réseau privé.
doctor-vpn-on = Un VPN est activé ({ $name }). Il peut masquer le réseau de votre maison.
doctor-vpn-on-fix = Désactivez le VPN pendant le transfert, puis réessayez.
doctor-old-laptop-not-pairing = L’ancien ordinateur n’affiche peut-être pas son code.
doctor-old-laptop-not-pairing-fix = Sur l’ancien ordinateur, ouvrez PCTwin et indiquez que c’est votre ancien ordinateur.
doctor-different-networks = Les ordinateurs sont peut-être sur des réseaux différents.
doctor-different-networks-fix = Connectez les deux ordinateurs au même Wi-Fi.
doctor-guest-network = Ce réseau empêche peut-être les appareils de se voir, comme souvent les Wi-Fi d’invités, d’hôtel ou de bureau.
doctor-guest-network-fix = Utilisez le Wi-Fi de votre maison, ou le partage de connexion de votre téléphone.
doctor-probe-interfaces = PCTwin n’a pas pu lister les connexions réseau de cet ordinateur.
doctor-probe-hears-itself = PCTwin n’a pas pu vérifier si cet ordinateur peut être trouvé sur le réseau.
doctor-probe-local-network = PCTwin n’a pas pu vérifier le réglage de réseau local du Mac.
doctor-probe-network-profiles = PCTwin n’a pas pu vérifier comment Windows traite ce réseau.
doctor-hotspot = Toujours pas ? Activez le partage de connexion de votre téléphone et connectez-y les deux ordinateurs. Le transfert passe directement entre les ordinateurs, pas par les données de votre téléphone.
windows-make-other-administrator = Vérifiez que la personne qui garde l’ordinateur y a un compte administrateur.
windows-sign-in-other-administrator = Déconnectez-vous, puis connectez-vous avec cet autre compte administrateur. Un compte ne peut pas se supprimer lui-même.
windows-open-other-users = Ouvrez Paramètres, puis Comptes, puis Autres utilisateurs.
windows-remove-account = Choisissez votre compte, puis Supprimer.
windows-delete-account-and-data = Choisissez Supprimer le compte et les données. Les comptes des autres restent tels quels.
windows-sign-out-of-everything = Déconnectez-vous d’abord de vos apps et de vos comptes sur l’ancien ordinateur.
windows-open-recovery = Ouvrez Paramètres, puis Système, puis Récupération.
windows-reset-this-pc = Choisissez Réinitialiser ce PC.
windows-remove-everything = Choisissez Supprimer tout.
windows-clean-the-drive = Modifiez les paramètres pour que le disque soit nettoyé, pas seulement vidé. Cela prend quelques heures.
windows-wait-until-setup-screen = Attendez que l’ordinateur affiche son écran de configuration. Il est alors prêt à être donné.
macos-make-other-administrator = Vérifiez que la personne qui garde le Mac y a un compte administrateur.
macos-sign-in-other-administrator = Fermez la session, puis ouvrez-la avec cet autre compte administrateur. Un compte ne peut pas se supprimer lui-même.
macos-open-users-and-groups = Ouvrez Réglages Système, puis Utilisateurs et groupes.
macos-delete-user = Choisissez le bouton d’information à côté de votre compte, puis Supprimer l’utilisateur.
macos-delete-home-folder = Choisissez de supprimer le dossier de départ.
macos-sign-out-of-apple-account = Déconnectez-vous d’abord de votre compte Apple sur l’ancien Mac, pour qu’il soit libre pour son prochain propriétaire.
macos-open-transfer-or-reset = Ouvrez Réglages Système, puis Général, puis Transférer ou réinitialiser.
macos-erase-all-content-and-settings = Choisissez Effacer contenu et réglages, et suivez les étapes.
macos-older-mac-use-recovery = Si votre Mac ne le propose pas (Mac plus anciens), démarrez-le en mode de récupération macOS, effacez le disque avec Utilitaire de disque, puis réinstallez macOS.
macos-wait-until-setup-screen = Attendez que le Mac affiche son écran de configuration. Il est alors prêt à être donné.
linux-sign-in-other-administrator = Connectez-vous avec un autre compte administrateur.
linux-open-users = Ouvrez Paramètres, puis Utilisateurs.
linux-remove-user-and-files = Choisissez votre compte, puis Supprimer l’utilisateur, et choisissez de supprimer ses fichiers.
linux-no-built-in-reset = Linux n’a pas de réinitialisation intégrée qui efface le disque.
linux-drive-maker-secure-erase = Utilisez l’outil d’effacement sécurisé du fabricant du disque, ou l’effacement propre à l’ordinateur dans ses réglages de démarrage, s’il en a un.
linux-reinstall-from-usb = Installez ensuite un système neuf depuis une clé USB.
any-sign-in-other-administrator = Connectez-vous avec un autre compte administrateur sur l’ancien ordinateur.
any-use-system-account-settings = Supprimez votre compte avec les réglages de comptes du système lui-même.
any-use-system-reset = Utilisez la réinitialisation propre à l’ordinateur qui efface le disque. L’aide du fabricant explique comment.
any-drive-maker-secure-erase = S’il n’y en a pas, utilisez l’outil d’effacement sécurisé du fabricant du disque.

# 2.9 deliverables
guard-undo-removes-if-package = PCTwin la supprime si elle venait d'un paquet d'installation ; sinon, supprimez-la vous-même
guard-undo-keeps-newer = L'annulation garde la version récente : l'ancienne ne peut pas revenir
compat-outcome-works = Testé : fonctionne
compat-outcome-works-with-steps = Testé : fonctionne, avec quelques étapes de votre part
compat-outcome-partly = Testé : certaines parties fonctionnent
compat-outcome-not-supported = Testé : PCTwin ne peut pas l'installer ici
compat-outcome-not-tested = Pas encore testé
compat-aspect-install = Installation
compat-aspect-data-import = Transfert de ses données
compat-aspect-settings = Ses réglages
compat-aspect-launch = Ouverture
compat-aspect-undo = Annulation
compat-result-works = Fonctionne
compat-result-guided = Fonctionne avec votre aide
compat-result-not-supported = Ne fonctionne pas
compat-result-not-applicable = Inutile
compat-result-not-tested = Non testé
compat-step-sign-in = Connectez-vous
compat-step-licence = Saisissez la clé de licence
compat-step-import-in-app = Importez vos données dans l'application
compat-step-relink = Indiquez-lui de nouveau vos fichiers
compat-step-restart = Redémarrez l'ordinateur
compat-how-lab = Testé par PCTwin sur des ordinateurs neufs
compat-how-real-laptops = Testé sur de vrais ordinateurs
work-what-linked-files = Fichiers liés
work-what-font = Police
work-what-archive = L'archive s'ouvre
work-what-database = La base de données s'ouvre
work-what-task = Une chose que vous vérifiez vous-même
work-label-machine-checked = Vérifié par PCTwin
work-label-person-confirmed = Vous l'avez vérifié
work-label-still-needs-attention = Reste à vérifier
work-state-not-run-yet = Pas encore vérifié
work-state-all-there = Tout est là
work-state-missing = Il manque quelque chose
work-state-damaged = Incomplet
work-state-not-there = Absent
work-state-waiting-for-you = En attente de votre réponse
work-state-you-said-no = Vous avez dit que cela n'a pas marché
work-state-outside-project = Liens hors de son dossier : non vérifiés
work-state-could-not-read = Lecture impossible
work-state-not-supported = PCTwin ne sait pas encore vérifier ce type de fichier
work-state-partly-checked = Tout n'a pas été vérifié
work-problem-not-chosen = Aucun fichier choisi.
work-problem-too-many = Trop de vérifications à la fois. Retirez-en d'abord.
work-problem-no-such-check = Cette vérification n'existe plus.
work-problem-stale = La liste a changé. Regardez-la de nouveau.
work-problem-bad-name = Saisissez un nom de 200 caractères au plus.
work-problem-wrong-kind = Cette réponse ne correspond pas à cette vérification.
work-problem-failed = Un problème est survenu. Réessayez.
repair-waits-old-laptop = L'ancien ordinateur doit être connecté
repair-waits-internet = Internet est nécessaire
repair-waits-administrator = Demande l’accord d’un administrateur
repair-waits-installer-tool = L'installateur d'applications du système est nécessaire
repair-waits-space = Il faut plus de place sur cet ordinateur
repair-waits-close-app = L'application doit être fermée
repair-problem-nothing-to-repair = Rien de ce qui est choisi n'est à réparer.
repair-problem-stale = La liste a changé. Regardez-la de nouveau.
repair-problem-busy = PCTwin est occupé. Réessayez quand il aura fini.
repair-problem-failed = Un problème est survenu. Rien d'autre n'a été modifié.
changed-why-app-updated = L’application a été mise à jour après votre accord
changed-why-app-removed = L’application a été supprimée après votre accord
changed-why-destination-changed = Le dossier de destination a changé
changed-why-project-edited = Il a été modifié sur l’ancien ordinateur portable après votre accord
changed-step-install = Installation
changed-step-settings = Réglages
changed-step-app-data = Données d’applis
changed-step-files = Fichiers
safe-stop-busy-moving = Transfert des fichiers
safe-stop-busy-rebuilding = Installation des applications et des réglages
safe-stop-busy-app-data = Données d’applis
safe-stop-busy-undoing = Annulation d'un déménagement
kit-part-move-record = Le relevé de votre déménagement
kit-part-results = Le résultat de chaque élément
kit-part-before-wipe-list = Ce qui n'est encore que sur l'ancien ordinateur
kit-part-file-data = Vos fichiers
kit-part-setup-steps = Les étapes pour réinstaller les applications
kit-part-recovery-tool = Un outil de restauration

# 2.8 screens
report-group-apps = Applications
report-group-settings = Réglages
report-group-special = Données d’applications et autres choses
report-group-people = Autres personnes
report-count-apps = { $count ->
        [one] { $count } appli
        [many] { $count } d’applis
       *[other] { $count } applis
    }
report-count-settings = { $count ->
        [one] { $count } groupe de réglages
        [many] { $count } de groupes de réglages
       *[other] { $count } groupes de réglages
    }
report-count-special = { $count ->
        [one] { $count } élément
        [many] { $count } d’éléments
       *[other] { $count } éléments
    }
report-count-people = { $count ->
        [one] { $count } personne
        [many] { $count } de personnes
       *[other] { $count } personnes
    }
report-beyond-done = Tout est fait et vérifié
report-beyond-some = { $done } sur { $total } faits et vérifiés
report-beyond-left = { $count ->
        [one] { $count } non déplacé : vous l’avez laissé de côté
        [many] { $count } non déplacés : vous les avez laissés de côté
       *[other] { $count } non déplacés : vous les avez laissés de côté
    }
report-beyond-more = { $count ->
        [one] { $count } autre n’est pas affiché ici.
        [many] { $count } autres ne sont pas affichés ici.
       *[other] { $count } autres ne sont pas affichés ici.
    }
report-here = { $count ->
        [one] { $count } application était déjà sur cet ordinateur. PCTwin l’a laissée telle quelle.
        [many] { $count } applications étaient déjà sur cet ordinateur. PCTwin les a laissées telles quelles.
       *[other] { $count } applications étaient déjà sur cet ordinateur. PCTwin les a laissées telles quelles.
    }
progress-ready-title = Prêt à utiliser
progress-ready-body = { $count ->
        [one] { $groups } est copié et vérifié. Vous pouvez l’ouvrir sur cet ordinateur pendant que le reste arrive.
        [many] { $groups } sont copiés et vérifiés. Vous pouvez les ouvrir sur cet ordinateur pendant que le reste arrive.
       *[other] { $groups } sont copiés et vérifiés. Vous pouvez les ouvrir sur cet ordinateur pendant que le reste arrive.
    }
finish-strip = Terminez votre transfert
finish-title = Terminez votre transfert
finish-loading = Examen de votre transfert.
finish-retry = Réessayer
finish-back-to-list = Retour à la liste
finish-open = Ouvrir
finish-open-name = Ouvrir { $title }
finish-more-not-shown = Autres non affichés : { $count }
finish-problem-no-move = PCTwin n’a aucun transfert à terminer sur cet ordinateur portable. Rien n’a été modifié.
finish-problem-changed = Ce qui est affiché n’est plus à jour. Regardez à nouveau avant de répondre. Rien n’a été modifié.
finish-problem-not-allowed = Cela n’est pas possible. Rien n’a été modifié.
finish-problem-busy = Un transfert ou une autre étape est en cours. Attendez la fin, puis réessayez. Rien n’a été modifié.
finish-problem-failed = Un problème est survenu sur cet ordinateur portable. Rien n’a été modifié. Réessayez.
finish-headline-ready-for-selected-work = Tout ce que vous avez choisi est sur cet ordinateur portable
finish-headline-ready-for-selected-work-body = PCTwin a trouvé sur cet ordinateur portable chaque élément que vous avez choisi et l’a vérifié autant qu’il le peut. Cela ne garantit pas que toutes les applications fonctionnent comme avant.
finish-headline-finished-with-tasks = Le transfert est terminé, et il reste des tâches
finish-headline-finished-with-tasks-body = Certaines choses ont besoin de vous, ou n’ont pas été copiées. Elles sont listées sur cette page.
finish-headline-data-preserved = Tout est ici, mais certains éléments demandent une clé pour s’ouvrir
finish-headline-data-preserved-body = Tout est sur cet ordinateur portable. Une clé ou un code de récupération est nécessaire pour ouvrir une partie, et PCTwin n’a pas pu le vérifier ici.
finish-headline-handover-unresolved = L’ancien ordinateur portable n’est pas encore prêt à être remis
finish-headline-handover-unresolved-body = Cet ordinateur portable est prêt. La liste avant de quitter l’ancien ordinateur portable a encore des points ouverts.
finish-headline-nothing-moved = Rien n’a été choisi pour le transfert
finish-headline-nothing-moved-body = Aucun fichier, application ni réglage n’a été choisi, donc rien n’a été copié.
finish-tally-title = Ce qui a été transféré
finish-tally-selected = Choisis : { $count }
finish-tally-done = Copiés et vérifiés : { $count }
finish-tally-needs-action = Ont besoin de vous : { $count }
finish-tally-failed = Non copiés : { $count }
finish-tally-waiting = Encore en attente : { $count }
finish-tally-not-reachable = Ancien ordinateur portable injoignable : { $count }
finish-tally-no-result-yet = Pas encore de résultat : { $count }
finish-tally-skipped = Ignorés par vous : { $count }
finish-tally-left-out = Laissés de côté : { $count }
finish-tally-accepted-loss = Laissés volontairement : { $count }
finish-others = Éléments d’autres personnes, seulement comptés et jamais affichés : { $count }
finish-after-title = Vérifié maintenant sur cet ordinateur portable
finish-after-body = PCTwin les a comptés sur l’ancien ordinateur portable, puis comptés à nouveau sur cet ordinateur portable.
finish-after-count = { $onNew } sur { $atScan } sont sur cet ordinateur portable
finish-after-all = Tout est là
finish-after-short = Il en manque
finish-group-photos = Photos
finish-group-videos = Vidéos
finish-group-music = Musique
finish-group-documents = Documents
finish-group-other-files = Autres fichiers
finish-group-apps = Applications
finish-group-settings = Réglages
finish-note-parental-controls = Le contrôle parental et les réglages de sécurité familiale ne sont pas transférés. Configurez-les à nouveau sur cet ordinateur portable.
finish-note-not-everywhere = Certains endroits de l’ancien ordinateur portable n’ont pas été examinés.
finish-note-results-refused = Certains résultats n’ont pas été acceptés, car cet ordinateur portable ne les a pas montrés comme terminés.
finish-note-undo-open = L’annulation reste possible pour ce transfert. Elle se ferme définitivement si vous effacez l’ancien ordinateur portable.
finish-open-title = Pas terminé
finish-open-body = Voici vos éléments qui demandent encore de l’attention.
finish-standing-copied = Copié
finish-standing-reinstalled = Réinstallé
finish-standing-converted = Converti
finish-standing-needs-action = A besoin de vous
finish-standing-failed = Non copié
finish-standing-waiting = En attente
finish-standing-not-reachable = Ancien ordinateur portable injoignable
finish-standing-no-result-yet = Pas encore de résultat
finish-standing-skipped-by-user = Ignoré par vous
finish-standing-left-out = Laissé de côté
finish-access-key-not-checked = Une clé ou un code pour l’ouvrir n’a pas été vérifié ici.
finish-access-not-on-new-laptop = Il n’est pas sur cet ordinateur portable.
finish-search-title = Où PCTwin a regardé sur l’ancien ordinateur portable
finish-search-searched = A regardé dans : { $places }
finish-search-not-searched = N’a pas regardé dans : { $places }
finish-search-all = PCTwin a regardé dans tous les endroits qu’il connaît. Il ne peut pas dire qu’il n’y a rien ailleurs.
finish-old-checked = Dernière vérification de l’ancien ordinateur portable : { $when }
finish-old-never = L’ancien ordinateur portable n’a pas été vérifié depuis le transfert.
finish-steps-title = Étapes suivantes
finish-step-setup = Terminer la configuration
finish-step-setup-detail = Applications qui demandent que vous vous connectiez ou terminiez une étape.
finish-step-goodbye = Avant de quitter l’ancien ordinateur portable
finish-step-goodbye-detail = Ce qui n’est encore que sur l’ancien ordinateur portable.
finish-step-catchup = Copier les changements récents
finish-step-catchup-detail = Copier ce qui a changé sur l’ancien ordinateur portable depuis le transfert.
finish-step-handover = Remettre l’ancien ordinateur portable
finish-step-handover-detail = Retirer votre compte ou effacer l’ancien ordinateur portable, avec les outils du système lui-même.
finish-step-undo = Annuler ce transfert
finish-step-undo-detail = Retirer des applications et remettre les réglages.
finish-step-cleanup = Nettoyer
finish-step-cleanup-detail = Retirer de cet ordinateur portable les fichiers temporaires de PCTwin.
finish-step-report = Partager le rapport
finish-step-report-detail = Enregistrer un rapport simple. Vous choisissez ce qu’il contient.
finish-step-doctor = Vérifier la connexion
finish-step-doctor-detail = Trouver pourquoi les deux ordinateurs portables ne se connectent pas.
finish-setup-title = Terminer la configuration
finish-setup-body = Ces applications demandent une étape de votre part. Une coche enregistre ce que vous dites. Elle ne compte pas comme une vérification de PCTwin.
finish-setup-empty = Rien ne vous attend.
finish-setup-sign-in = Connectez-vous à { $name }
finish-setup-licence-key = Saisissez la clé de licence de { $name }
finish-setup-set-up-again = Configurez { $name } à nouveau
finish-setup-download-from-maker = Téléchargez { $name } depuis le site de son éditeur
finish-setup-permission = Donnez l’autorisation à { $name } quand il la demande
finish-setup-restart = Redémarrez cet ordinateur portable pour terminer { $name }
finish-setup-finish-by-hand = Terminez { $name } à la main
finish-setup-parts-missing = Certaines parties de { $name } ne sont pas arrivées. Envoyez-les à nouveau
finish-setup-export-first = Exportez d’abord depuis { $name } sur l’ancien ordinateur portable
finish-setup-relink = Reliez { $name } à votre compte à nouveau
finish-setup-look-at-it = Regardez { $name } et vérifiez que tout est correct
finish-setup-mark-open = Pas fait
finish-setup-mark-machine-checked = PCTwin a vérifié
finish-setup-mark-you-did-it = Vous avez dit que c’est fait
finish-setup-tick-hint = Cochez quand c’est fait.
finish-setup-save-row = Enregistrer la liste de vos applications
finish-setup-save-row-detail = Garder une liste de vos applications et réglages dans Mon historique PCTwin.
finish-save-title = Enregistrer la liste de vos applications
finish-save-body = PCTwin garde une liste de vos applications et réglages dans Mon historique PCTwin, sur cet ordinateur portable seulement. C’est une liste, pas une copie de vos fichiers.
finish-save-button = Enregistrer la liste
finish-save-saved = Enregistré dans Mon historique PCTwin. Applications : { $apps }. Réglages : { $settings }.
finish-save-earlier-title = Enregistré avant
finish-save-earlier = Enregistré : { $when }
finish-goodbye-title = Avant de quitter l’ancien ordinateur portable
finish-goodbye-body = Voici ce qui n’est encore que sur l’ancien ordinateur portable. Répondez pour chaque point. Rien n’est effacé ici. Effacer est une étape séparée.
finish-goodbye-saved = Cette liste est enregistrée sur cet ordinateur portable.
finish-goodbye-only-on-old-laptop = Seulement sur l’ancien ordinateur portable : { $count }
finish-goodbye-key-not-checked = Éléments qui demandent une clé ou un code non vérifié : { $count }
finish-goodbye-tied-to-old-laptop = Liés à l’ancien ordinateur portable : { $count }
finish-goodbye-do-first-on-old-laptop = À faire d’abord sur l’ancien ordinateur portable : { $count }
finish-goodbye-steps-not-finished = Étapes non terminées : { $count }
finish-goodbye-not-searched = Endroits non examinés : { $count }
finish-goodbye-old-laptop-not-reachable = L’ancien ordinateur portable n’a pas pu être joint
finish-goodbye-special-sign-in = { $names } : connectez-vous à nouveau
finish-goodbye-special-relink = { $names } : reliez-le à votre compte à nouveau
finish-goodbye-special-export-first = { $names } : exportez d’abord ce dont vous avez besoin depuis l’ancien ordinateur portable
finish-goodbye-special-look-at-it = { $names } : regardez-le et vérifiez que tout est correct
finish-goodbye-places = Endroits : { $places }
finish-goodbye-did-it = C’est fait
finish-goodbye-accept = J’accepte de le perdre
finish-goodbye-accept-warning = Si vous acceptez, cela reste seulement sur l’ancien ordinateur portable. Quand l’ancien ordinateur portable sera effacé, cela sera perdu.
finish-goodbye-cannot-answer = Seule une nouvelle vérification de l’ancien ordinateur portable peut régler cela. Connectez-le et vérifiez à nouveau.
finish-goodbye-res-open = Pas encore de réponse
finish-goodbye-res-person-did-it = Vous avez dit que c’est fait
finish-goodbye-res-accepted-loss = Vous avez accepté de le perdre
finish-goodbye-answer-label = Votre réponse pour : { $title }
finish-goodbye-keep-title = Gardez l’ancien ordinateur portable intact jusqu’au
finish-goodbye-keep-body = Date suggérée : { $date }. D’ici là, ne l’effacez pas, au cas où vous auriez besoin de quelque chose.
finish-goodbye-keep-label = Garder jusqu’au
finish-goodbye-keep-use = Utiliser la date suggérée
finish-goodbye-keep-save = Enregistrer cette date
finish-goodbye-keep-set = Gardé jusqu’au : { $date }
finish-goodbye-keep-none = Aucune date choisie pour l’instant
finish-goodbye-blocked = L’ancien ordinateur portable ne peut pas encore être remis. Encore ouvert : { $count }.
finish-goodbye-blocked-unchecked = L’ancien ordinateur portable n’a pas été vérifié depuis le transfert. Vérifiez-le d’abord à nouveau.
finish-goodbye-clear = Tout a une réponse et l’ancien ordinateur portable a été vérifié. Vous pouvez demander une remise. C’est une étape séparée.
finish-goodbye-handover = Remettre l’ancien ordinateur portable
finish-goodbye-check-again = Vérifier à nouveau l’ancien ordinateur portable
finish-catchup-title = Copier les changements récents
finish-catchup-body = Voyez ce qui a changé sur l’ancien ordinateur portable depuis le transfert. Vous examinez chaque changement. PCTwin envoie seulement ceux que vous choisissez.
finish-catchup-purpose-check = Seulement regarder l’ancien ordinateur portable
finish-catchup-purpose-check-detail = Rien n’est copié. Votre liste avant de partir est mise à jour.
finish-catchup-purpose-catch-up = Regarder et copier ce qui a changé
finish-catchup-purpose-catch-up-detail = Vous voyez d’abord les changements et choisissez ce qui est envoyé.
finish-catchup-armed = Prêt. Connectez l’ancien ordinateur portable comme pour le transfert. Cette page montre les changements quand l’ancien ordinateur portable répond.
finish-catchup-connect = Connecter l’ancien ordinateur portable
finish-catchup-waiting = En attente de l’ancien ordinateur portable.
finish-catchup-unreadable = PCTwin n’a pas pu lire la réponse de l’ancien ordinateur portable. Rien n’a été modifié. Réessayez.
finish-catchup-changes-title = Ce qui a changé
finish-catchup-check-only = Rien n’est envoyé. Cette liste met seulement à jour ce que PCTwin sait de l’ancien ordinateur portable.
finish-catchup-none = Rien n’a changé sur l’ancien ordinateur portable.
finish-catchup-others = Changements d’éléments d’autres personnes, seulement comptés : { $count }
finish-catchup-choose-label = Que doit faire PCTwin avec { $title } ?
finish-catchup-send-label = Envoyer { $title }
finish-catchup-nothing-to-send = Rien à envoyer
finish-change-new = Nouveau sur l’ancien ordinateur
finish-change-updated = Modifié sur l’ancien ordinateur portable
finish-change-both-changed = Modifié sur les deux ordinateurs portables
finish-change-gone-on-new-laptop = Retiré de cet ordinateur portable
finish-change-gone-on-old-laptop = Retiré de l’ancien ordinateur portable
finish-change-cannot-tell = PCTwin ne peut pas le dire
finish-choice-none = Ne pas envoyer
finish-choice-send = L’envoyer à nouveau
finish-choice-keep-both = Garder les deux copies
finish-choice-keep-new-laptop-version = Garder la version de cet ordinateur portable
finish-catchup-approve = Envoyer les changements choisis : { $count }
finish-catchup-sent = Envoyé pour copie. Seuls les changements que vous avez choisis sont copiés.
finish-handover-title = Remettre l’ancien ordinateur portable
finish-handover-body = Chaque choix est une décision séparée. Rien ne se passe tant que vous ne confirmez pas sur sa propre page.
finish-handover-remove-title = Retirer mon compte de l’ancien ordinateur portable
finish-handover-remove-detail = Seulement votre compte. Les comptes des autres personnes restent.
finish-handover-erase-title = Effacer l’ancien ordinateur portable
finish-handover-erase-detail = Tout ce qui se trouve sur le disque de l’ancien ordinateur portable.
finish-handover-preview-remove = Retirer mon compte
finish-handover-preview-erase = Effacer l’ancien ordinateur portable
finish-handover-removes = Ce qui sera retiré
finish-handover-keeps = Ce qui reste
finish-handover-keeps-none = Rien n’est conservé.
finish-handover-route = Comment cela se fait
finish-handover-route-body = PCTwin ne le fait pas lui-même. C’est l’outil du système qui le fait, et PCTwin vous y conduit.
finish-handover-continue = Continuer pour confirmer
finish-handover-not-offered = Ce n’est pas proposé pour l’instant
finish-handover-refused-open = Points encore ouverts dans votre liste avant de partir : { $count }
finish-handover-refused-nothing-done = Rien n’a été fait.
finish-handover-to-list = Aller à la liste avant de partir
finish-route-system-windows = Windows
finish-route-system-macos = macOS
finish-route-system-linux = Linux
finish-route-unknown-step = Suivez la prochaine étape que le système vous montre.
finish-hnote-ssd-not-secure-wipe = Supprimer des fichiers sur un SSD n’est pas un effacement sécurisé.
finish-hnote-does-not-cancel-subscriptions = Retirer un compte n’annule pas les abonnements et ne vous déconnecte pas des autres endroits.
finish-hnote-others-stay = Les comptes de tous les autres restent tels quels.
finish-hnote-cannot-be-undone = Cela ne peut pas être annulé.
finish-hnote-closes-undo = Continuer ferme définitivement l’annulation pour ce transfert.
finish-hnote-sign-in-as-another-administrator = Un compte ne peut pas se retirer lui-même. Connectez-vous d’abord à l’ancien ordinateur portable avec un autre compte administrateur.
finish-hnote-family-group-first = Si le compte fait partie d’un groupe familial Microsoft, retirez-le d’abord du groupe familial en ligne.
finish-hnote-not-a-certificate = PCTwin enregistre ce que vous avez demandé et ce que vous lui dites qu’il s’est passé. Ce n’est pas un certificat que quelque chose a été effacé.
finish-hnote-encrypt-first-helps = Activer le chiffrement du disque avant la réinitialisation rend ce qui reste beaucoup plus difficile à lire.
finish-refused-wrong-device = Cette étape n’est pas disponible sur cet ordinateur portable.
finish-refused-undo-not-closed = L’annulation reste possible pour ce transfert. Il faut d’abord la fermer.
finish-refused-old-laptop-not-checked = L’ancien ordinateur a été vérifié pour la dernière fois il y a plus de 15 minutes. Vérifiez-le à nouveau d’abord.
finish-refused-list-out-of-date = La liste avant de partir n’est plus à jour. Ouvrez-la à nouveau.
finish-refused-list-not-clear = La liste avant de partir a encore des points ouverts.
finish-refused-no-write-exclusion = PCTwin n’a pas pu s’assurer que rien ne change en ce moment sur l’ancien ordinateur portable.
finish-refused-no-such-account = Votre compte est introuvable sur l’ancien ordinateur portable.
finish-refused-other-people-use-it = D’autres personnes utilisent ce compte, donc PCTwin ne le retirera pas.
finish-refused-not-qualified = Cet ancien ordinateur portable ne peut pas être remis de cette façon.
finish-refused-no-fresh-confirmation = Une nouvelle confirmation est nécessaire.
finish-refused-wrong-confirmation = Cette confirmation ne correspond pas à ce qui est affiché maintenant. Regardez à nouveau.
finish-refused-changed-since-confirmed = Quelque chose a changé après votre confirmation. Regardez à nouveau et confirmez encore.
finish-refused-expired = La confirmation a expiré. Regardez à nouveau et confirmez encore.
finish-refused-already-used = Cette confirmation a déjà été utilisée.
finish-refused-nothing-to-remove = Il n’y a rien à retirer.
finish-confirm-title-remove-my-account = Confirmer : retirer mon compte
finish-confirm-title-erase-old-laptop = Confirmer : effacer l’ancien ordinateur portable
finish-confirm-target = Ceci va modifier : { $target }
finish-confirm-consequences = Ce que cela signifie
finish-confirm-tick-remove-my-account = Je comprends que le compte { $target } sera retiré de l’ancien ordinateur portable et que cela ne peut pas être annulé.
finish-confirm-tick-erase-old-laptop = Je comprends que tout ce qui se trouve sur { $target } sera effacé et que cela ne peut pas être annulé.
finish-confirm-go = Confirmer et ouvrir l’outil du système
finish-confirm-cancel = Revenir sans rien faire
finish-confirm-needs-tick = Le bouton fonctionne une fois la case ci-dessus cochée.
finish-started-title = Suivez les étapes du système lui-même
finish-started-body = Les étapes ci-dessous sont celles du système, pas de PCTwin. PCTwin n’en affiche pas sa propre version.
finish-started-opened = PCTwin a ouvert la page du système.
finish-started-not-opened = PCTwin n’a pas pu ouvrir la page du système. Ouvrez-la vous-même et suivez les étapes.
finish-started-undo-closed = L’annulation est maintenant fermée pour ce transfert.
finish-says-title = Dites à PCTwin ce qui s’est passé
finish-says-label = Qu’avez-vous vu ?
finish-says-choose = Choisissez ce que vous avez vu
finish-says-done = J’ai terminé les étapes
finish-says-partly = J’ai terminé une partie des étapes
finish-says-cancelled-part-way = J’ai annulé en cours de route
finish-says-not-started = Je n’ai pas commencé
finish-says-cannot-tell = Je ne sais pas
finish-says-record = Enregistrer ce que j’ai vu
finish-recorded-person-says-done = Enregistré : vous avez dit que c’est terminé. C’est votre parole. Ce n’est pas un certificat.
finish-recorded-partial = Enregistré : seulement une partie a été faite.
finish-recorded-interrupted = Enregistré : cela a été interrompu.
finish-recorded-cancelled-after-some-effects = Enregistré : cela a été annulé après quelques changements.
finish-recorded-not-started = Enregistré : cela n’a pas commencé.
finish-recorded-inconclusive = Enregistré : vous n’avez pas pu savoir ce qui s’est passé.
finish-recorded-time = Enregistré : { $when }
finish-undo-title = Vérifier avant d’annuler
finish-undo-body = Voici ce que l’annulation fait et ce qu’elle laisse. Rien ne se passe tant que vous ne choisissez pas Commencer l’annulation.
finish-undo-row-removes = Applications installées par PCTwin : l’annulation les retire
finish-undo-row-yours = Applications qui sont maintenant à vous : l’annulation les laisse. Retirez-les vous-même si vous voulez.
finish-undo-row-settings = Réglages remis comme avant : { $count }
finish-undo-row-appdata = Données d’application remises pour ces applications
finish-undo-row-files = Fichiers copiés par le transfert : { $count }
finish-undo-limit-files = Seulement avec l’ancien ordinateur portable connecté, et seulement les copies que personne n’a modifiées depuis.
finish-undo-limit-apps = Seulement tant que chaque application est encore la version installée par PCTwin.
finish-undo-keeps = Ce que l’annulation garde : les applications qui sont maintenant à vous, et tout ce que vous avez modifié après le transfert.
finish-undo-closed = L’annulation est fermée pour ce transfert. Elle ne peut pas être utilisée.
finish-undo-nothing = Il n’y a ni application ni réglage à annuler pour ce transfert.
finish-undo-start = Commencer l’annulation
finish-cleanup-title = Nettoyer
finish-cleanup-body = Chaque question est posée séparément. Rien n’est retiré sans votre confirmation.
finish-cleanup-temp = Fichiers temporaires de PCTwin
finish-cleanup-temp-detail = Fichiers que PCTwin a créés sur cet ordinateur portable pendant son travail.
finish-cleanup-model = Modèle et données enregistrées de PCTwin
finish-cleanup-model-detail = Fichiers d’aide que PCTwin garde sur cet ordinateur portable pour aller plus vite.
finish-cleanup-look = Voir
finish-cleanup-removes = Sera retiré
finish-cleanup-keeps = Sera conservé
finish-cleanup-keeps-body = Ils ressemblent seulement à ceux de PCTwin, ou sont en cours d’utilisation.
finish-cleanup-nothing = Il n’y a rien à retirer.
finish-cleanup-tick = Retirer les éléments listés sous Sera retiré
finish-cleanup-confirm = Les retirer
finish-cleanup-done = Retirés : { $removed }. Conservés : { $kept }. Impossible à retirer : { $failed }.
finish-doctor-title = Vérifier la connexion
finish-doctor-body = PCTwin examine cet ordinateur portable pour trouver pourquoi les deux ordinateurs portables ne se connectent pas.
finish-doctor-role-label = Quel ordinateur portable est-ce ?
finish-doctor-role-old = L’ancien ordinateur portable
finish-doctor-role-new = Le nouvel ordinateur portable
finish-doctor-run = Lancer la vérification
finish-doctor-running = Vérification en cours.
finish-doctor-found = Ce que PCTwin a trouvé
finish-doctor-none = PCTwin n’a trouvé aucun problème sur cet ordinateur portable. Cela ne prouve pas que la connexion fonctionnera.
finish-doctor-sure = Certain
finish-doctor-likely = Probable
finish-doctor-fix = Que faire
finish-doctor-not-checked = PCTwin n’a pas pu vérifier
finish-notchecked-unknown = Une vérification n’a pas pu être faite sur cet ordinateur portable.
finish-report-title = Partager le rapport
finish-report-body = Choisissez ce que le rapport contient. Rien n’est inclus sans votre choix. Rien n’est envoyé. Vous enregistrez le texte que vous lisez, et vous le partagez vous-même.
finish-report-choices = Inclure dans le rapport
finish-report-file-names = Noms de fichiers
finish-report-app-names = Noms d’applications
finish-report-account-id = Identifiant du compte
finish-report-preview = Le rapport tel qu’il sera enregistré
finish-report-withheld = Gardé hors du rapport : noms { $names }, secrets { $secrets }, identifiants de compte { $ids }.
finish-report-save = Enregistrer le rapport
finish-report-saved = Enregistré dans : { $path }
finish-report-not-saved = Aucun fichier n’a été enregistré.
finish-report-feedback = Pour dire à l’équipe PCTwin ce qui s’est passé, utilisez Aide, puis Commentaires. Aucun compte n’est nécessaire, et rien n’est envoyé automatiquement.
finish-report-help = Ouvrir l’aide
finish-report-text-title = Rapport de transfert PCTwin
finish-report-text-unnamed-file = Un fichier
finish-report-text-unnamed-app = Une application
finish-report-text-account = Identifiant du compte : { $id }
finish-report-text-others = Éléments d’autres personnes non terminés, seulement comptés : { $count }
finish-report-text-goodbye = Liste avant de partir : { $total } points, { $open } encore ouverts
finish-report-text-lines = Éléments non terminés
rescue-eyebrow = Récupérer un ordinateur
rescue-title = Récupérer un ordinateur
rescue-lead = Récupérez les fichiers d’un ordinateur qui ne démarre plus. Branchez son disque sur cet ordinateur, puis choisissez-le ici.
rescue-checking = Recherche des disques branchés.
rescue-failed = PCTwin n’a pas pu lister les disques branchés sur cet ordinateur. Vérifiez le câble, puis vérifiez à nouveau.
rescue-empty = Aucun disque branché. Branchez le disque de l’ancien ordinateur avec un adaptateur USB, puis vérifiez à nouveau.
rescue-looks-like-laptop = Semble être le disque d’un ordinateur
rescue-choose = Choisir { $name }
rescue-check-again = Vérifier à nouveau
rescue-locked-title = Si le disque est verrouillé
rescue-locked-windows = Si le disque est verrouillé par BitLocker, Windows demande sa clé de récupération dans sa propre fenêtre quand vous ouvrez le disque.
rescue-wait-title = Lecture de { $drive }
rescue-wait-body = PCTwin parcourt les dossiers de { $drive }. Rien n’est modifié sur le disque.
rescue-accept-title = Fichiers trouvés sur { $drive }
rescue-accept-body = Acceptez, et PCTwin les copie sur cet ordinateur. Rien n’est modifié sur le disque.
rescue-progress-title = Copie des fichiers de { $drive }
rescue-safe = Le disque est seulement lu. Rien n’y est modifié.
rescue-outcome-title = Fichiers copiés depuis { $drive }
plan-net-title = Ce qui a besoin d’internet
plan-net-none = Rien dans ce transfert n’a besoin d’internet.
plan-net-downloads =
    { $count ->
        [one] PCTwin télécharge { $count } application depuis internet.
        [many] PCTwin télécharge { $count } applications depuis internet.
       *[other] PCTwin télécharge { $count } applications depuis internet.
    }
plan-net-sizes = La taille de chaque téléchargement n’est connue qu’une fois celui-ci terminé.
plan-net-signins =
    { $count ->
        [one] Vous vous reconnectez en ligne à { $count } chose.
        [many] Vous vous reconnectez en ligne à { $count } choses.
       *[other] Vous vous reconnectez en ligne à { $count } choses.
    }
plan-room-title = Place nécessaire en plus de vos fichiers
plan-room-body = Au moment le plus chargé, PCTwin a besoin d’environ { $installs } pour les applications et d’environ { $appData } pour leurs données, en plus de vos fichiers.
compat-strip = Ce qui a été testé
compat-title = { $name } : ce qui a été testé
compat-open = Détails
compat-open-name = Voir ce qui a été testé pour { $name }
compat-list-title =
    { $count ->
        [one] Résultat du test de { $count } application
        [many] Résultat du test de { $count } applications
       *[other] Résultat du test de { $count } applications
    }
compat-aspects-title = Chaque partie
compat-steps-title = Ce que vous ferez
compat-steps-none = Rien à faire de votre côté.
compat-never-checked = Personne n’a encore testé cette application. PCTwin l’essaiera et vous dira ce qui s’est passé.
compat-checked-on = Vérifié le { $date }.
compat-version = Version testée : { $version }
work-strip = Vérification du travail
work-title = Vérifiez que vous pouvez travailler ici
work-body = Ajoutez les fichiers, les polices et les tâches dont vous avez besoin sur cet ordinateur. PCTwin regarde ce qu’il peut lire. Vous dites si le reste a fonctionné. Rien n’est ouvert ni lancé.
work-empty = Aucune vérification pour l’instant. Ajoutez ci-dessous un fichier, une police ou une tâche.
work-looked-at = { $checked } sur { $of } examinés.
work-missing = Manque :
work-and-more =
    { $count ->
        [one] Et { $count } de plus.
        [many] Et { $count } de plus.
       *[other] Et { $count } de plus.
    }
work-not-looked-at = Non examiné :
work-answer-worked = Ça a fonctionné
work-answer-worked-name = Ça a fonctionné : { $name }
work-answer-not = Ça n’a pas fonctionné
work-answer-not-name = Ça n’a pas fonctionné : { $name }
work-remove = Retirer
work-remove-name = Retirer { $name }
work-run = Vérifier maintenant
work-add-title = Ajouter une vérification
work-add-file = Choisir un fichier
work-add-file-hint = Un projet, une archive ou une base de données. PCTwin le lit et ne le modifie jamais.
work-font-label = Une police, par son nom
work-add-font = Ajouter la police
work-task-label = Une tâche que vous faites vous-même
work-task-hint = Par exemple : imprimer une carte d’embarquement. Vous dites si cela a fonctionné.
work-add-task = Ajouter la tâche
repair-strip = Réparer ce qui n’a pas fini
repair-title =
    { $count ->
        [one] Réparer cet élément
        [many] Réparer ces { $count } éléments
       *[other] Réparer ces { $count } éléments
    }
repair-title-loading = Réparer ce qui n’a pas fini
repair-body =
    { $kept ->
        [one] Seuls les éléments ci-dessous sont réessayés. { $kept } élément qui avait déjà fonctionné reste tel quel.
        [many] Seuls les éléments ci-dessous sont réessayés. { $kept } éléments qui avaient déjà fonctionné restent tels quels.
       *[other] Seuls les éléments ci-dessous sont réessayés. { $kept } éléments qui avaient déjà fonctionné restent tels quels.
    }
repair-nothing = Rien à réparer.
repair-can-now = Peut être essayé maintenant
repair-waiting = En attente
repair-fixed =
    { $count ->
        [one] { $count } élément réparé.
        [many] { $count } éléments réparés.
       *[other] { $count } éléments réparés.
    }
repair-run =
    { $count ->
        [one] Réessayer { $count } élément
        [many] Réessayer { $count } éléments
       *[other] Réessayer { $count } éléments
    }
changed-strip = Changements depuis votre accord
changed-title =
    { $count ->
        [one] { $count } chose a changé après votre accord
        [many] { $count } choses ont changé après votre accord
       *[other] { $count } choses ont changé après votre accord
    }
changed-body = Seuls ces éléments demandent de nouveau votre accord. Tout le reste reste approuvé.
changed-affects = Concerne : { $steps }
changed-held =
    { $count ->
        [one] { $count } étape attend votre réponse.
        [many] { $count } étapes attendent votre réponse.
       *[other] { $count } étapes attendent votre réponse.
    }
changed-leave-out = Laisser ceci de côté
changed-approve = Approuver de nouveau
safe-stop-strip = Mise à jour
safe-stop-title = En attente d’un moment sûr
safe-stop-body = PCTwin ne se met pas à jour et ne se retire pas en plein transfert. Il attend que plus rien ne soit à moitié fait.
safe-stop-failed = PCTwin n’a pas pu savoir si c’est sûr. Réessayez.
safe-stop-ready = Rien n’est en train de changer. PCTwin peut se mettre à jour ou être retiré maintenant.
safe-stop-waiting-for = Il attend la fin de ceci :
safe-stop-nothing-going-on = Aucun transfert, réglage ou annulation en cours.
safe-stop-check = Vérifier à nouveau
# Help feedback, safe stop links, catch-up and repair routes
feedback-strip = Commentaires
feedback-title = Comment le transfert s’est-il passé ?
feedback-body = Les commentaires sont facultatifs et n’affectent pas les transferts gratuits.
feedback-rating-label = Évaluation
feedback-rating-none = Sans évaluation
feedback-rating-very-good = Très bien
feedback-rating-good = Bien
feedback-rating-needs-work = À améliorer
feedback-words-label = Qu’est-ce qui pourrait l’améliorer ?
feedback-words-hint = Facultatif. N’indiquez ni noms, ni mots de passe, ni noms de fichiers.
feedback-preview-title = Voici tout ce qui serait partagé
feedback-preview-rating = Évaluation : { $rating }
feedback-preview-empty = Rien pour l’instant. Choisissez une évaluation ou écrivez quelques mots.
feedback-sends-nothing = PCTwin n’envoie rien de lui-même. Aucun compte n’est nécessaire. Pour partager ce texte, copiez-le, puis ouvrez getpctwin.com pour joindre l’équipe PCTwin.
feedback-copy = Copier les commentaires
feedback-copied = Commentaires copiés.
feedback-copy-failed = Les commentaires n’ont pas pu être copiés. Sélectionnez le texte ci-dessus et copiez-le vous-même.
feedback-open-site = Ouvrir getpctwin.com
help-home-feedback = Commentaires
help-home-update = Mettre à jour ou supprimer PCTwin
safe-stop-see = Voir ce qui est en attente
history-repair = Réparer les éléments
kit-strip = Kit de récupération
kit-title = Enregistrer un kit de récupération
kit-body = Un kit de récupération est un petit relevé de ce transfert, enregistré dans un dossier de votre choix, par exemple sur une clé USB. Il montre ce qui s’est passé et ce qui restait à faire, même sans cet ordinateur.
kit-lacks-sentence = Le kit ne contient pas vos fichiers eux-mêmes. Il ne peut pas rendre des fichiers manquants.
kit-not-saved = Aucun dossier n’a été choisi. Rien n’a été enregistré.
kit-saved-to = Enregistré dans :
kit-files-title = Fichiers du kit
kit-holds-title = Ce que contient le kit
kit-lacks-title = Ce que le kit ne contient pas
kit-save = Choisir un dossier et enregistrer
kit-save-again = Enregistrer à nouveau
kit-history = Kit de récupération
finish-step-repair =
    { $count ->
        [one] Réparer cet élément
        [many] Réparer ces { $count } éléments
       *[other] Réparer ces { $count } éléments
    }
finish-step-repair-detail = Réessayer seulement ce qui n’a pas fini. Ce qui a fonctionné reste.
finish-step-work = Vérifiez que vous pouvez travailler ici
finish-step-work-detail = Vérifier que les fichiers, polices et tâches dont vous avez besoin sont là.
finish-step-kit = Enregistrer un kit de récupération
finish-step-kit-detail = Garder un petit relevé de ce transfert dans un dossier de votre choix.
space-box-title = Cet ordinateur n’a pas assez de place pour tout
space-box-choice = { $count ->
        [one] Laisser sur votre ancien ordinateur { $count } fichier ({ $size }) que vous n’avez pas utilisé depuis le plus longtemps
        [many] Laisser sur votre ancien ordinateur { $count } fichiers ({ $size }) que vous n’avez pas utilisés depuis le plus longtemps
       *[other] Laisser sur votre ancien ordinateur { $count } fichiers ({ $size }) que vous n’avez pas utilisés depuis le plus longtemps
    }
space-box-kept = Rien n’est supprimé. Ils restent sur votre ancien ordinateur.
space-box-short = Même ainsi, il manque { $size }. Décochez certains dossiers ou choisissez un autre endroit pour eux.
space-box-fits = S’ils restent sur votre ancien ordinateur, tout le reste tient.
finish-handover-check-old = Vérifier à nouveau l’ancien ordinateur
finish-handover-check-armed = PCTwin est prêt à vérifier l’ancien ordinateur. Ouvrez PCTwin sur l’ancien ordinateur et connectez les deux ordinateurs comme pour un transfert. Puis choisissez Regarder à nouveau.
finish-handover-look-again = Regarder à nouveau
qs-where-title = Où les enregistrer ?
qs-where-body = { $count ->
        [one] { $count } fichier, { $size }. Rien n’est enregistré avant votre choix.
        [many] { $count } fichiers, { $size }. Rien n’est enregistré avant votre choix.
       *[other] { $count } fichiers, { $size }. Rien n’est enregistré avant votre choix.
    }
qs-where-folder = Dossier : { $folder }
qs-where-details = Plus de détails
qs-where-path = Emplacement complet : { $path }
qs-where-save = Enregistrer ici
qs-where-choose = Choisir un autre dossier
qs-where-stop = Arrêter

# 2.9 destination changed after approval
outcome-landed-moved = Ce dossier a été déplacé pendant le transfert. Vos fichiers sont dedans, là où il se trouve maintenant.
move-place-changed-title = Un dossier a changé après votre accord
move-place-changed-body = Un dossier de destination a été déplacé ou remplacé après votre accord. Rien n’a été copié. Vérifiez le dossier, puis recommencez le transfert.

# Quick Send stage 1
qs-refusal-not-paired = Les ordinateurs ne sont plus connectés. Connectez-les à nouveau.
qs-refusal-not-waiting = Rien n'attend cela pour le moment.
qs-refusal-failed = Cela n'a pas marché. Réessayez.
qs-refusal-note-empty = Écrivez d'abord quelque chose à envoyer.
qs-refusal-note-too-long = Cette note est trop longue. Restez sous 1 000 caractères.
qs-refusal-note-too-many-lines = Cette note a trop de lignes. Gardez-en 20 au plus.
qs-refusal-note-hidden = Cette note contient un caractère qui ne peut pas être affiché sans risque. Écrivez-la à nouveau.
qs-refusal-no-link = Il n'y a aucun lien à ouvrir.
qs-refusal-not-found = Ceci n'est plus là.
qs-refusal-restored = Cet ordinateur a été restauré ou copié. Connectez à nouveau les deux ordinateurs avec un code.
qs-refusal-bad-name = Donnez-lui un nom de 40 lettres au plus.
qs-refusal-store-unavailable = PCTwin n'a pas pu atteindre le coffre à clés de cet ordinateur. Réessayez après vous être connecté.
qs-refusal-too-many-connections = Vous avez trop de connexions enregistrées. Oubliez-en une d'abord.
qs-refusal-folder-runs-at-sign-in = Ce dossier lance des programmes à l'ouverture de session. Choisissez un autre dossier.
qs-refusal-folder-shell-start-up = Ce dossier lance des choses à l'ouverture d'un terminal. Choisissez un autre dossier.
qs-refusal-folder-holds-keys = Ce dossier contient des clés. Choisissez un autre dossier.
qs-refusal-folder-system = C'est un dossier du système lui-même. Choisissez un autre dossier.
qs-refusal-folder-not-plain = Ce n'est pas un dossier ordinaire. Choisissez un autre dossier.
qs-refusal-folder-not-the-same = Ce n'est pas le dossier choisi la dernière fois. Choisissez-le à nouveau.
qs-refusal-not-nearby = L'autre ordinateur n'est pas à proximité. Il est peut-être éteint ou sur un autre réseau.
qs-refusal-over-cap = Les envois en attente prendraient trop de place. Annulez-en un d'abord.
qs-refusal-no-room = Il n'y a pas assez d'espace libre sur cet ordinateur.
qs-refusal-bad-days = Choisissez de 1 à 30 jours.
qs-refusal-bad-recipients = Choisissez chaque personne une seule fois, 20 au plus.
qs-refusal-too-many-files = Il y a trop de fichiers. Choisissez-en moins.
qs-refusal-changed-while-copying = Un fichier a changé pendant la copie. Réessayez.
qs-note-copy = Copier
qs-note-open = Ouvrir
qs-note-link-to = Ce lien mène à { $host }.
qs-note-sent = La note est arrivée.
qs-remember-ask = Se souvenir de cette connexion ?
qs-remember-body = La prochaine fois, vous pourrez envoyer sans code. Cela désigne le même ordinateur qu'avant, pas une personne.
qs-remember-fallback = Sur cet ordinateur, la clé est gardée dans un fichier de votre compte.
qs-known-same-device = Le même appareil qu'avant
qs-known-restored = Cet ordinateur a été restauré ou copié. Connectez-vous à nouveau.
qs-known-last-used = Dernière utilisation : { $date }
qs-known-forget = Oublier
qs-known-forget-body = La clé et toutes les autorisations sont supprimées. Les fichiers déjà envoyés restent où ils sont.
qs-known-allow-folder = Autoriser les prochains envois dans un dossier
qs-known-allow-body = Les fichiers vont dans un dossier nommé PCTwin Quick Send à l'intérieur. Cela prend fin seul après 30 jours.
qs-known-wants-to-send = { $label } veut vous envoyer ceci.
qs-known-delivered = Des fichiers de { $label } sont arrivés dans { $folder }.
qs-waiting-days =
    { $count ->
        [one] Attend 1 jour
        [many] Attend { $count } jours
       *[other] Attend { $count } jours
    }
qs-waiting-files =
    { $count ->
        [one] 1 fichier en attente
        [many] { $count } fichiers en attente
       *[other] { $count } fichiers en attente
    }
qs-waiting-newer = Une version plus récente existe. Vérifiez-la à nouveau.
qs-waiting-on = L'envoi ne se fait que lorsque cet ordinateur est allumé.
qs-waiting-cancel = Annuler et supprimer la copie

# Quick Send stage 1 fixes
qs-known-cannot-tell = PCTwin ne peut pas savoir s’il s’agit du même ordinateur.
qs-remember-replaces = Avez-vous déjà connecté cet ordinateur ? Choisissez-le et l’ancienne connexion sera oubliée.
qs-remember-replaces-suggested = C’est peut-être encore { $label }. Le remplacer ?
qs-remember-replace = Remplacer
qs-remember-keep-both = Non, garder les deux

# 2.6 wiring
reason-open-in-app = Une appli avait ce fichier ouvert sur l’ancien ordinateur, donc PCTwin ne l’a pas copié (une copie faite pendant que l’appli écrit pourrait être abîmée). Fermez cette appli sur l’ancien ordinateur, puis lancez un nouveau transfert qui inclut ce fichier.
report-open-in = Ouvert dans { $app }
fullmove-close-files = Ces fichiers sont ouverts dans une appli sur cet ordinateur, donc PCTwin ne peut pas encore les copier sans risque. Fermez l’appli qui les utilise, puis choisissez Réessayer. Si vous continuez sans eux, ils restent sur cet ordinateur et le rapport les indique.
choose-online-only =
    { NUMBER($count) ->
        [one] Apporter aussi le fichier gardé seulement en ligne dans { $provider } (1 fichier, { $size }). Il est d’abord téléchargé sur cet ordinateur, puis remis seulement en ligne.
        [many] Apporter aussi les fichiers gardés seulement en ligne dans { $provider } ({ $count } de fichiers, { $size }). Ils sont d’abord téléchargés sur cet ordinateur, puis remis seulement en ligne.
       *[other] Apporter aussi les fichiers gardés seulement en ligne dans { $provider } ({ $count } fichiers, { $size }). Ils sont d’abord téléchargés sur cet ordinateur, puis remis seulement en ligne.
    }
choose-online-only-unnamed = un service de stockage en ligne de cet ordinateur
choose-online-only-metered = Aussi sur une connexion qui peut facturer les données (le partage de connexion d’un téléphone)
choose-online-only-not-here = Si certains de vos fichiers sont gardés seulement en ligne, ils restent dans votre stockage en ligne. PCTwin ne peut pas encore en apporter une copie depuis cet ordinateur.
kept-online-copied =
    { NUMBER($count) ->
        [one] 1 fichier gardé seulement en ligne a été téléchargé et copié.
        [many] { $count } de fichiers gardés seulement en ligne ont été téléchargés et copiés.
       *[other] { $count } fichiers gardés seulement en ligne ont été téléchargés et copiés.
    }
kept-online-safe =
    { NUMBER($count) ->
        [one] 1 fichier gardé seulement en ligne n’a pas été copié. Il est toujours gardé en ligne : ouvrez-le sur le nouvel ordinateur depuis l’appli qui le garde.
        [many] { $count } de fichiers gardés seulement en ligne n’ont pas été copiés. Ils sont toujours gardés en ligne : ouvrez-les sur le nouvel ordinateur depuis l’appli qui les garde.
       *[other] { $count } fichiers gardés seulement en ligne n’ont pas été copiés. Ils sont toujours gardés en ligne : ouvrez-les sur le nouvel ordinateur depuis l’appli qui les garde.
    }
kept-online-still-here =
    { NUMBER($count) ->
        [one] 1 fichier téléchargé prend encore { $size } sur cet ordinateur, car son appli cloud ne l’a pas encore remis seulement en ligne. Pour libérer la place, faites un clic droit dessus et choisissez Libérer de l’espace.
        [many] { $count } de fichiers téléchargés prennent encore { $size } sur cet ordinateur, car leur appli cloud ne les a pas encore remis seulement en ligne. Pour libérer la place, faites un clic droit sur chacun et choisissez Libérer de l’espace.
       *[other] { $count } fichiers téléchargés prennent encore { $size } sur cet ordinateur, car leur appli cloud ne les a pas encore remis seulement en ligne. Pour libérer la place, faites un clic droit sur chacun et choisissez Libérer de l’espace.
    }
kept-online-paused-metered = Le téléchargement s’est arrêté, car cette connexion peut facturer les données. Le reste n’a pas été copié et est toujours gardé en ligne.
kept-online-paused-space = Le téléchargement s’est arrêté, car cet ordinateur manque de place. Le reste n’a pas été copié et est toujours gardé en ligne.
plan-own-folder = Mettre les affaires de { $laptop } dans leur propre dossier
plan-own-folder-help = Cet ordinateur a déjà des affaires d’un autre ancien ordinateur. Un dossier par ordinateur les garde séparées.
plan-own-folder-name = De { $laptop }
plan-own-folder-name-unnamed = De l’autre ordinateur
plan-downloads =
    { NUMBER($count) ->
        [one] 1 appli est téléchargée depuis internet pour être installée sur cet ordinateur.
        [many] { $count } d’applis sont téléchargées depuis internet pour être installées sur cet ordinateur.
       *[other] { $count } applis sont téléchargées depuis internet pour être installées sur cet ordinateur.
    }
plan-wait-free-wifi = Attendre un Wi-Fi gratuit avant de télécharger
plan-wait-free-wifi-help = Si la connexion peut facturer les données au moment de télécharger les applis, PCTwin attend jusqu’à 20 minutes un Wi-Fi gratuit. Les applis qu’il n’a pas pu télécharger sont listées pour que vous les installiez plus tard.
plan-reuse-installers =
    { NUMBER($count) ->
        [one] Utiliser le programme d’installation trouvé sur l’ancien ordinateur pour 1 appli, au lieu de la télécharger
        [many] Utiliser les programmes d’installation trouvés sur l’ancien ordinateur pour { $count } d’applis, au lieu de les télécharger
       *[other] Utiliser les programmes d’installation trouvés sur l’ancien ordinateur pour { $count } applis, au lieu de les télécharger
    }
plan-reuse-installers-help = Chacun est d’abord vérifié, comme un téléchargement : il doit être signé par l’éditeur de l’appli.
fullmove-close-files-title = Fermez les applis qui utilisent ces fichiers
choose-online-only-title = Fichiers gardés seulement en ligne
kept-online-title = Fichiers gardés seulement en ligne
kept-online-downloaded = { $size } téléchargés
plan-own-folder-called = Le dossier s’appelle { $folder }.
accept-own-folder = Les affaires de l’autre ordinateur vont dans leur propre dossier, { $folder }. Vous pouvez désactiver cela dans les détails.
plan-downloads-title = Applis depuis internet
plan-connection-free = La connexion de cet ordinateur ne facture pas les données en ce moment.
plan-connection-may-charge = La connexion de cet ordinateur peut facturer les données en ce moment, par exemple le partage de connexion d’un téléphone.
plan-connection-not-known = PCTwin ne peut pas savoir si la connexion de cet ordinateur facture les données.
fullmove-rebuilding-wait-wifi = Si cette connexion peut facturer les données, PCTwin attend jusqu’à 20 minutes un Wi-Fi gratuit avant de télécharger les applis. Connectez-vous à un Wi-Fi gratuit pour continuer plus tôt.

# Quick Send stage 1 screens
qs-note-card-title = Envoyer une note ou un lien
qs-note-card-body = Quelques lignes de texte, ou un lien web.
qs-home-known = Connexions mémorisées
qs-home-waiting = Envois en attente
qs-note-title = Envoyer une note ou un lien
qs-note-intro = Tapez quelques lignes, ou collez un lien web. Seul ce que vous tapez ici est envoyé.
qs-note-label = Note ou lien
qs-note-hint = Jusqu'à 20 lignes et 1 000 caractères.
qs-note-never-opens = Un lien envoyé ne s'ouvre que si la personne qui le reçoit choisit Ouvrir.
qs-note-sending-title = Envoi de la note
qs-note-sending-body = Veuillez patienter pendant que l'ordinateur destinataire la reçoit.
qs-note-sent-title = La note est arrivée
qs-note-sent-body = L'ordinateur destinataire l'a reçue. PCTwin ne peut pas savoir si elle a été lue.
qs-note-received-title = Une note est arrivée
qs-note-received-link-title = Un lien est arrivé
qs-note-from = De { $label }.
qs-note-from-code = De l'ordinateur connecté avec un code.
qs-note-link-safety = Il s'ouvre dans votre navigateur seulement si vous choisissez Ouvrir.
qs-note-copied = Copié.
qs-note-copy-failed = PCTwin n'a pas pu le copier. Sélectionnez le texte et copiez-le vous-même.
qs-remember-name = Nom de cette connexion
qs-remember-name-hint = Jusqu'à 40 lettres, pour le retrouver plus tard.
qs-remember-save = Mémoriser
qs-remember-done = Mémorisée sous le nom { $label }. Ses fichiers vous sont toujours demandés d'abord.
qs-known-title = Connexions mémorisées
qs-known-intro = Les ordinateurs auxquels vous pouvez envoyer sans code. Chacun désigne le même ordinateur qu'avant, pas une personne.
qs-known-empty = Aucune connexion n'est encore mémorisée. Après un envoi ou une réception avec un code, PCTwin vous demande s'il faut mémoriser cet ordinateur.
qs-known-expires = Oubliée d'elle-même le { $date } si elle n'est pas utilisée d'ici là.
qs-known-rename = Renommer
qs-known-rename-label = Nouveau nom
qs-known-rename-save = Enregistrer le nom
qs-known-forget-title = Oublier { $label } ?
qs-known-forget-confirm = L'oublier
qs-known-folder-on = Ses fichiers vont dans { $folder }, dans un dossier nommé PCTwin Quick Send.
qs-known-folder-ends = Cela prend fin le { $date }.
qs-known-folder-ended = Cela a pris fin le { $date }. Ses fichiers vous sont de nouveau demandés d'abord.
qs-known-renew = Renouveler 30 jours
qs-known-next-title = Dossier pour la prochaine fois
qs-known-next-on = { $folder } est déjà choisi quand il vous envoie des fichiers. Vous pouvez toujours le changer à chaque fois.
qs-known-next-none = Non défini. Quand il vous envoie des fichiers, Téléchargements est déjà choisi, ou vous choisissez un autre dossier à chaque fois.
qs-known-next-use = Utiliser le dernier dossier
qs-known-next-clear = Ne plus l'utiliser
qs-known-in-more = et { $count } de plus
qs-known-in-next = Utiliser ce dossier pour { $label } la prochaine fois
qs-known-in-decline = Refuser
qs-known-delivered-counts = { $arrived } sur { $total } copiés et vérifiés.
qs-send-to = Envoyer à { $name }
qs-to-title = Qui doit les recevoir ?
qs-to-intro = Choisissez des ordinateurs mémorisés, ou connectez un ordinateur avec un code.
qs-to-known = Ordinateurs mémorisés
qs-to-code = Utiliser un code
qs-to-when = Quand
qs-to-now = Maintenant
qs-to-now-hint = L'ordinateur destinataire doit être allumé et à proximité.
qs-to-later = Quand il est à proximité
qs-to-several = Chaque ordinateur les reçoit quand il est à proximité.
qs-to-days = Durée d'attente
qs-to-each-asks = L'ordinateur destinataire doit choisir où enregistrer, sauf s'il a autorisé les envois dans un dossier.
qs-to-approve-hint = Vous approuvez ces fichiers tels qu'ils sont, ces ordinateurs et cette durée. PCTwin garde une copie des fichiers à envoyer. Chaque ordinateur doit choisir où enregistrer, sauf s'il a autorisé les envois dans un dossier.
qs-to-send-now = Envoyer maintenant
qs-to-approve = Approuver cet envoi
qs-waiting-title = Envois en attente
qs-waiting-intro = Des fichiers dont PCTwin garde une copie, à envoyer quand chaque ordinateur est à proximité.
qs-waiting-empty = Rien n'est en attente.
qs-waiting-approved = Approuvé le { $date }
qs-waiting-until = Attend jusqu'au { $date }
qs-waiting-ended = Terminé le { $date }. Rien de plus n'est envoyé.
qs-waiting-to-unknown = Une connexion oubliée
qs-waiting-state-waiting = En attente
qs-waiting-state-delivered = { $arrived } sur { $total } copiés et vérifiés
qs-waiting-state-needs = Action nécessaire : { $count }
qs-waiting-state-declined = Refusé le { $date }
qs-waiting-state-failed = Non copiés : { $count }
qs-waiting-renew-label = Durée d'attente à partir de maintenant
qs-waiting-renew = Renouveler
qs-waiting-cancel-title = Annuler cet envoi en attente ?
qs-waiting-cancel-body = PCTwin supprime sa copie, et rien de plus n'est envoyé. Les ordinateurs qui ont déjà les fichiers les gardent. Vos propres fichiers restent tels quels.
qs-waiting-keep = Le garder

# Quick Send more
qs-session-add-files = Ajouter des fichiers
qs-session-send-to = Envoyer à { $name }
qs-session-send-back = Renvoyer des fichiers
qs-session-end = Terminer
qs-session-ended = Cet envoi est fini. Plus rien n’arrive.
qs-sends-waiting = En attente de son tour
qs-sends-sending = Envoi en cours
qs-sends-paused = En pause
qs-send-pause = Pause
qs-send-resume = Reprendre
qs-send-cancel = Annuler

# Open sends: Add files, Send to, several laptops at once, questions that wait, a live file being copied
qs-session-ended-name = { $name } : cet envoi est fini. Plus rien n’arrive.
qs-sends-action-name = { $what } : { $name }
qs-sends-title = Vos envois
qs-sends-intro = PCTwin envoie à trois ordinateurs au plus en même temps, et un envoi après l’autre vers le même ordinateur. Les autres attendent leur tour.
qs-sends-empty = Rien n’est envoyé en ce moment.
qs-sends-ended-title = Envois terminés
qs-sends-stopped = Il a été arrêté avant la fin.
qs-sends-result-whole = Copiés et vérifiés
qs-sends-result-partial = Certains non copiés
qs-sends-not-sent = Non envoyé à { $name }.
qs-sends-failed = PCTwin n’a pas pu modifier cet envoi. Réessayez.
qs-session-offer-waiting = { $name } vous envoie d’autres fichiers.
qs-session-offer-show = Choisir où enregistrer
qs-to-now-several-hint = Chaque ordinateur doit être allumé et à proximité. PCTwin envoie à trois au plus en même temps ; les autres attendent leur tour.
qs-asking-files = { $label } veut vous envoyer des fichiers.
qs-asking-note = { $label } vous a envoyé une note.
qs-asking-later = PCTwin vous pose la question quand vous quittez cette page.
qs-asking-show = Choisir où enregistrer
qs-asking-show-note = Afficher la note
progress-current-open-in = Pendant que PCTwin copie { $name }, { $app } ne peut pas y enregistrer de modifications. Si { $app } indique que le fichier est utilisé, attendez la fin de la copie.

# 2.6 waiting for free Wi-Fi
fullmove-rebuilding-waiting-wifi = { $minutes ->
        [one] En attente d’un Wi-Fi gratuit avant de télécharger les applis : encore 1 minute au plus
        [many] En attente d’un Wi-Fi gratuit avant de télécharger les applis : encore { $minutes } minutes au plus
       *[other] En attente d’un Wi-Fi gratuit avant de télécharger les applis : encore { $minutes } minutes au plus
    }

# 2.5 files held by an app nobody can name
fullmove-live-wait-title = En attente de fichiers libres
fullmove-live-wait-body = Ces fichiers sont ouverts dans une application sur cet ordinateur. PCTwin vérifie de nouveau toutes les 30 secondes, pendant { $minutes } minutes au plus. S’ils sont encore ouverts, le transfert continue sans eux et le rapport les indique.
fullmove-live-wait-stop = Arrêter le transfert

# 2.5 games
special-note-saves-in-launcher-cloud = Le lanceur de votre jeu garde ces sauvegardes en ligne. Elles reviennent quand vous vous connectez à lui sur votre nouvel ordinateur portable, donc PCTwin ne les copie pas sauf si vous les cochez. La copie de l'ancien ordinateur portable reste telle quelle.
special-note-save-may-not-work-here = Ces sauvegardes viennent d'un autre type de système. PCTwin les garde sur votre nouvel ordinateur portable, mais le jeu ne pourra peut-être pas les ouvrir.
special-note-store-protected-saves = La boutique garde ces sauvegardes pour elle-même, donc PCTwin ne peut pas les copier. Elles reviennent quand vous vous connectez à la boutique sur votre nouvel ordinateur portable et installez le jeu.
special-note-mod-programs-held = Certains modules de ce jeu sont des programmes. PCTwin les liste, mais ne les copie pas et ne les active pas. Récupérez-les auprès de leur créateur.
special-note-launcher-verify = Installez le lanceur sur votre nouvel ordinateur portable et connectez-vous. Utilisez ensuite sa propre étape de vérification ou de localisation pour qu'il trouve et vérifie vos jeux. PCTwin ne copie les jeux installés que si vous le choisissez.
choose-wifi-title = Réseaux Wi-Fi enregistrés
choose-wifi-admin = Les réseaux cochés partent avec leur mot de passe, et le nouveau portable s’y connecte de lui-même. Pour lire les mots de passe, ce portable peut demander l’accord d’un administrateur dans sa propre fenêtre.
choose-wifi-ask-admin = Lire les mots de passe Wi-Fi demande un administrateur de ce portable : aucun réseau n’est donc coché. Pour les apporter, demandez à la personne qui s’occupe de ce portable, puis cochez-les : ce portable lui demande son accord dans sa propre fenêtre.
choose-wifi-open = Sans mot de passe
choose-wifi-organisation = Configuré par une organisation : reconnectez-vous sur le nouveau portable avec ses informations
choose-wifi-other = PCTwin ne déplace pas ce type de réseau : reconnectez-vous sur le nouveau portable
move-wait-wifi-title = Lecture de vos mots de passe Wi-Fi enregistrés
move-wait-wifi-body = Ce portable peut demander l’accord d’un administrateur dans sa propre fenêtre. Répondez-y pour apporter les réseaux Wi-Fi cochés. Rien n’est modifié sur ce portable.
fullmove-group-wifi = Réseaux Wi-Fi enregistrés : { $count }
fullmove-wifi-step-add = PCTwin l’ajoute ici avec son mot de passe, envoyé chiffré après votre accord.
fullmove-wifi-step-add-by-profile = Vous l’ajoutez ici une seule fois, dans Réglages Système, pour tous ces réseaux. PCTwin ouvre la page.
fullmove-wifi-step-skipped = Non coché sur l’ancien portable : il y reste.
fullmove-wifi-step-organisation = Configuré par une organisation : reconnectez-vous ici avec ses informations.
fullmove-wifi-step-not-this-kind = PCTwin ne déplace pas ce type de réseau : reconnectez-vous ici.
fullmove-wifi-step-no-password = Son mot de passe n’a pas pu être lu sur l’ancien portable : tapez-le une fois en vous connectant à ce réseau ici.
fullmove-wifi-why-organisation = Configuré par une organisation : reconnectez-vous avec ses informations.
fullmove-wifi-why-not-this-kind = PCTwin ne déplace pas ce type de réseau : reconnectez-vous ici.
fullmove-wifi-why-no-password = Son mot de passe n’a pas pu être lu sur l’ancien portable : tapez-le une fois en vous connectant à ce réseau.
fullmove-wifi-why-did-not-arrive = Il n’est pas arrivé de l’ancien portable : tapez son mot de passe une fois en vous connectant à ce réseau.
fullmove-wifi-why-already-here = Un réseau de ce nom était déjà enregistré ici : PCTwin l’a laissé tel quel. S’il ne se connecte pas, tapez son mot de passe une fois.
fullmove-wifi-why-not-added = Ce portable ne l’a pas ajouté : tapez son mot de passe une fois en vous connectant à ce réseau.
fullmove-wifi-why-install-profile = Non ajouté : le profil “Wi-Fi networks from your old laptop (PCTwin)” n’a pas été installé à temps dans Réglages Système. Tapez son mot de passe une fois en vous connectant à ce réseau.
fullmove-rebuilding-wifi = Ajout de vos réseaux Wi-Fi enregistrés
fullmove-rebuilding-wifi-profile = { $minutes ->
        [one] Pour ajouter vos réseaux Wi-Fi, installez le profil “Wi-Fi networks from your old laptop (PCTwin)” sur la page de Réglages Système ouverte par PCTwin : encore 1 minute au plus
        [many] Pour ajouter vos réseaux Wi-Fi, installez le profil “Wi-Fi networks from your old laptop (PCTwin)” sur la page de Réglages Système ouverte par PCTwin : encore { $minutes } minutes au plus
       *[other] Pour ajouter vos réseaux Wi-Fi, installez le profil “Wi-Fi networks from your old laptop (PCTwin)” sur la page de Réglages Système ouverte par PCTwin : encore { $minutes } minutes au plus
    }
report-group-wifi = Réseaux Wi-Fi
report-count-wifi = { $count ->
        [one] { $count } réseau
        [many] { $count } de réseaux
       *[other] { $count } réseaux
    }
