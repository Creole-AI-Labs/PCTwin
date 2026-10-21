# PCTwin screens, French (fr). Machine-translated ahead of time from ../en/main.ftl;
# a string missing here falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = Accueil PCTwin
language-label = Langue

footer-free = Transferts personnels gratuits. Aucun compte nécessaire.
footer-help = Aide

## F01 Welcome

welcome-eyebrow = Votre prochain chapitre vous attend.
welcome-title-line1 = Votre configuration.
welcome-title-line2 = Votre nouvel ordinateur portable.
welcome-body = Emportez vos fichiers, vos applis et vos réglages habituels pour le transfert.
welcome-start = Commencer un transfert

## F06 Old or new laptop

role-back = Retour
role-eyebrow = Commencer un transfert
role-title = Quel ordinateur portable est-ce ?
role-body = Ouvrez PCTwin sur les deux ordinateurs portables. Puis indiquez lequel est celui-ci.
role-old-title = Mon ancien ordinateur portable
role-old-body = Envoyer mes fichiers, applis et réglages.
role-new-title = Mon nouvel ordinateur portable
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

connect-prep-title = Rapprochez les deux ordinateurs portables.
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
peer-title = Voici votre autre ordinateur portable.
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
check-title = Vérifions la connexion.
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
check-speed-unknown = PCTwin n’a pas pu mesurer la connexion cette fois. Vérifiez à nouveau dans un instant.
check-space-unknown = PCTwin n’a pas pu lire l’espace libre sur le nouvel ordinateur portable.
check-power-unknown = PCTwin ne sait pas si { $label } est branché.
check-internet-unknown = PCTwin ne sait pas si cet ordinateur portable est en ligne. Ce n’est pas grave : la copie des fichiers n’a pas besoin d’internet.
check-again = Vérifier à nouveau
check-estimate-note = La durée estimée n’est pas une promesse.
check-continue = Choisir quoi transférer

## C09 Waiting for your other laptop

reconnect-title = En attente de votre autre ordinateur portable.
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
reconnect-details = Une reconnexion sûre
reconnect-details-body = Seuls ces deux ordinateurs portables peuvent se reconnecter, et ils utilisent à chaque fois une nouvelle connexion privée.

## C10 The connection has ended (dialog)

ended-title = La connexion est terminée.
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
pair-heads-up-title = Votre ordinateur peut vous poser une question.
pair-heads-up-local-network = Votre Mac va demander si PCTwin peut trouver des appareils sur votre réseau local. Choisissez Autoriser, pour que vos deux ordinateurs portables puissent se trouver.
pair-heads-up-firewall = Windows peut demander si PCTwin peut accepter des connexions. Choisissez Autoriser, pour que votre nouvel ordinateur portable puisse joindre celui-ci.
pair-heads-up-continue = Continuer

## C03 Old laptop: show the code

pair-old-eyebrow = Ancien ordinateur portable
pair-old-title = Connectons vos ordinateurs portables.
pair-old-body = Sur votre nouvel ordinateur portable, saisissez le code ci-dessous.
pair-old-next = Ensuite, vous choisirez le numéro affiché sur les deux ordinateurs portables.
pair-old-network = Gardez les deux ordinateurs portables sur le même Wi-Fi.
pair-old-waiting = En attente de votre nouvel ordinateur portable…
pair-old-starting = Préparation d’un code…
pair-old-label-title = Nom provisoire de cet ordinateur portable
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
pair-old-wait-title = Un nouveau code arrive.
pair-old-wait-body =
    { $seconds ->
        [one] Cet essai n’a pas fonctionné. Un nouveau code apparaît dans { $seconds } seconde.
        [many] Cet essai n’a pas fonctionné. Un nouveau code apparaît dans { $seconds } secondes.
       *[other] Cet essai n’a pas fonctionné. Un nouveau code apparaît dans { $seconds } secondes.
    }
pair-old-locked-title = Quelqu’un sur ce réseau a essayé de mauvais codes.
pair-old-locked-body = Après cinq mauvais codes, l’association s’est arrêtée pour protéger cet ordinateur portable. Rien n’a été partagé. Si c’était vous, choisissez Recommencer pour obtenir un nouveau code.
pair-old-locked-hotspot = Si vous ne savez pas qui c’était, connectez plutôt les deux ordinateurs portables au partage de connexion de votre téléphone, puis choisissez Recommencer.
pair-old-start-again = Recommencer
pair-old-timed-out = Aucun numéro n’a été choisi à temps, voici donc un nouveau code.
pair-old-declined = Les numéros ne correspondaient pas, rien n’a donc été partagé. Voici un nouveau code.
pair-old-left = Le nouvel ordinateur portable a arrêté l’association avant le choix d’un numéro, rien n’a donc été partagé. Voici un nouveau code.
pair-network-title = Choisissez votre Wi-Fi.
pair-network-hint = Choisissez le réseau de votre nouvel ordinateur portable.

## C05 Old laptop: match the number

pair-choose-eyebrow = Confirmer la connexion
pair-choose-title = Choisissez le numéro correspondant.
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
pair-new-search-failed-title = Impossible de chercher votre ancien portable.
pair-new-list-title = Lequel est votre ancien ordinateur portable ?
pair-new-list-body = Choisissez le nom affiché sur l’écran de votre ancien ordinateur portable.
pair-new-same-label = Un autre ordinateur portable à proximité porte ce nom. Vérifiez l’écran de l’ancien ordinateur.
pair-new-needs-update = Cet ordinateur a une version plus ancienne de PCTwin. Mettez PCTwin à jour dessus, puis recherchez à nouveau.
pair-new-none-title = Ancien ordinateur introuvable.
pair-new-none-body = Vérifiez que PCTwin est ouvert sur votre ancien ordinateur portable et affiche un code, et que les deux ordinateurs sont sur le même Wi-Fi. Puis relancez la recherche.
pair-new-search-again = Rechercher à nouveau
pair-new-code-title = Saisissez le code.
pair-new-code-body = Saisissez le code affiché sur { $label }.
pair-new-code-label = Code à six chiffres
pair-new-code-hint = Lisez-le sur l’écran de votre ancien ordinateur portable. Il change chaque minute.
pair-new-connect = Connecter
pair-new-connecting = Connexion…
pair-new-other-laptop = Choisir un autre ordinateur portable
pair-new-number-eyebrow = Confirmer la connexion
pair-new-number-title = Choisissez ce numéro sur votre ancien ordinateur portable.
pair-new-number-tile = Votre numéro
pair-new-number-label = Votre numéro est { $number }
pair-new-number-body = Votre ancien ordinateur portable affiche trois numéros. Choisissez celui-ci sur l’ancien.
pair-new-number-waiting = En attente de votre ancien ordinateur portable…

## Paired (both laptops)

pair-done-eyebrow = Tout est prêt
pair-done-title = Vos ordinateurs portables sont associés.
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
people-title = Choisissez le compte de chaque personne.
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
before-title = Avant de quitter cet ordinateur portable.
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

## S11 Get help with this task (a short problem code)

help-eyebrow = Aide et confidentialité
help-title = Obtenir de l’aide pour cette tâche.
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
scan-title = Recherche de votre configuration habituelle.
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
overview-title = Tout ce que vous avez choisi, en un coup d’œil.
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
files-title = Gardez ce qui compte pour vous.
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
dest-title = Donnez une place à chaque fichier.
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
plan-title = Approuvez le transfert que vous voulez.
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
plan-version = Version du plan { $revision }
plan-details = Plus de détails
plan-preview-title = Avant de commencer
plan-preview-fits = Ce portable a assez d’espace libre.
plan-preview-short = Ce portable a besoin de { $size } d’espace libre en plus. Libérez de l’espace avant de commencer, sinon le transfert s’arrêtera quand le disque sera plein.
plan-preview-space-unknown = PCTwin n’a pas pu lire l’espace libre de ce portable.
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
plan-blocked-someone-unmapped = Une personne de l’ancien ordinateur portable n’a pas encore de compte choisi sur le nouveau. Choisissez où vont les fichiers de chaque personne, puis revenez.
plan-blocked-account-used-twice = Deux personnes se retrouveraient dans le même compte sur le nouvel ordinateur portable. Donnez à chacune son propre compte.
plan-blocked-invalid = Ce plan ne peut pas être approuvé tel quel. Revenez en arrière et vérifiez vos choix.
plan-blocked-fix = Choisir les comptes

## Tray menu

tray-show = Afficher PCTwin
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

## X01 Move progress, compact

progress-eyebrow = Votre transfert
progress-title = Nous préparons votre arrivée.
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
progress-compact = Vue compacte
progress-expand = Voir tout le transfert
progress-stop = Arrêter le transfert

## X02 Move progress, expanded

progress-all-title = Voir tout le transfert.
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

## F09 Mini panel

mini-title = Votre transfert, à proximité.
mini-title-paused = Transfert en pause.
mini-what = transfert de vos fichiers sélectionnés
mini-progress =
    .aria-label = Progression du transfert
mini-pause = Pause
mini-resume = Reprendre
mini-open = Ouvrir le transfert
# Used when lane E reuses this panel for a Quick Send.
mini-title-send = Vos fichiers sont en route.
mini-what-send = envoi des fichiers sélectionnés
mini-open-send = Ouvrir l’envoi
mini-progress-send =
    .aria-label = Progression de l’envoi

## X05 Paused at a safe point

pause-title = En pause à un point sûr.
pause-title-finishing = Mise en pause à un point sûr…
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
outcome-title = Bienvenue sur votre nouvel ordinateur portable.
outcome-body = Voici ce qui est arrivé à tout ce que vous avez approuvé.
outcome-files-title = Fichiers sélectionnés
outcome-status-partial = Fait en partie
outcome-next-title = Que faire ensuite
outcome-old-safe = Tout ce qui n’est pas terminé reste listé ici. Rien n’a été retiré de votre ancien ordinateur portable.
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
move-get-help = Obtenir de l’aide
pickup-eyebrow = Votre transfert
pickup-title = Reprenez en toute sécurité.
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
pickup-done = Tout est en ordre. Associez à nouveau les portables pour continuer : ce qui est déjà ici est reconnu et n’est pas copié deux fois.
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
move-status-kept = Conservé
move-wait-eyebrow = Votre transfert
move-wait-scanning-title = Recherche de vos fichiers.
move-wait-scanning-body = Votre ancien portable parcourt vos dossiers. Il les lit seulement : rien n’est ouvert, modifié ou supprimé.
move-wait-approval-title = En attente du nouveau portable.
move-wait-approval-body = Vos fichiers sont prêts. Vérifiez le plan sur le nouveau portable et approuvez-le pour commencer.
move-wait-list-title = En attente de l’ancien portable.
move-wait-list-body = Votre ancien portable cherche vos fichiers. Ensuite, le plan s’affiche ici pour que vous l’approuviez. Cela peut prendre quelques minutes.
move-wait-starting-title = Début du transfert.
move-wait-starting-body = Le plan est approuvé. La copie commence dans un instant.
move-wait-admin-title = En attente d’un administrateur.
move-wait-admin-body = Ce portable demande l’accord d’un administrateur pour déplacer aussi les affaires des autres personnes. Répondez dans la fenêtre qu’il a ouverte.
move-others-not-approved = L’administrateur n’a pas donné son accord : seules vos affaires sont déplacées cette fois. Celles des autres restent en place.
move-others-failed = PCTwin n’a pas pu lancer ce qu’il faut pour déplacer les affaires des autres : seules les vôtres sont déplacées cette fois. Rien des leurs n’a changé.
move-others-not-here = Déplacer les affaires d’autres personnes depuis ce portable n’est pas encore prêt dans cette version : seules les vôtres sont déplacées.
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
move-lost-title = La connexion avec votre autre portable a été perdue.
move-lost-body = Les portables ne communiquent plus, le transfert n’a donc pas pu continuer. Vérifiez qu’ils sont allumés, éveillés et proches l’un de l’autre.
move-not-approved-title = Le transfert n’a pas commencé.
move-not-approved-old = Le plan n’a pas été approuvé sur le nouveau portable, rien n’a donc été copié.
move-not-approved-new = Vous n’avez pas approuvé le plan, rien n’a donc été copié.
move-failed-title = Un problème est survenu.
move-failed-body = Le transfert n’a pas pu continuer. Si cela se reproduit, Plus de détails, ci-dessous, indique ce qui s’est passé.
move-refused-title = Le transfert n’a pas pu commencer.
move-refused-not-paired = Vos portables ne sont plus associés. Associez-les à nouveau pour commencer le transfert.
move-refused-busy = Un transfert est déjà en cours sur ce portable.
move-refused-not-moving = Aucun transfert n’est en cours sur ce portable.
move-refused-plan-changed = Le plan a changé avant d’être approuvé. Associez à nouveau les portables pour voir le nouveau plan.
move-refused-failed = Les dossiers de ce portable n’ont pas pu être préparés. Associez à nouveau les portables pour réessayer.
drive-problem-title = Le transfert a besoin de vous.
drive-problem-body = Les copies enregistrées dans { $place } ne correspondaient pas aux originaux lors des vérifications de PCTwin, plusieurs fois. Le disque est peut-être défaillant, plein ou débranché : la copie y est en pause.
drive-problem-kept-title = Déjà copié et vérifié
drive-problem-waiting-title = Reste à copier dans { $place }
drive-problem-badge = En pause
drive-problem-fix = Vérifiez que le disque est branché et a de l’espace libre, puis continuez. Rien de ce que vous avez choisi n’est écarté pour contourner le problème. Vous pouvez aussi arrêter.
drive-problem-carry-on = Continuer

## R02 Your move, item by item (detailed report)

report-title = Votre transfert, élément par élément.
report-body = Ce qui est arrivé à chaque élément, pourquoi, d’où il vient et où il est allé.
report-filter-label = Afficher
report-col-item = Élément
report-col-result = Résultat
report-col-details = Vérification ou étape suivante
report-from = De : { $path }
report-to = Vers : { $path }
report-none = Aucun élément n’a ce résultat.
report-more-details = Plus de détails

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
reason-sign-in = C’est en place, mais vous devez vous connecter. Ouvrez l’élément et connectez-vous.
reason-activation = L’appli est en place, mais elle a besoin de sa licence sur le nouvel ordinateur portable. Ouvrez-la et activez-la avec votre licence ou votre compte.
reason-permission = Le nouvel ordinateur portable doit d’abord donner son autorisation. Ouvrez l’élément et choisissez Autoriser quand c’est demandé.
reason-online-only = Ce fichier est stocké seulement en ligne, il n’était donc pas sur l’ancien ordinateur portable pour être copié. Téléchargez-le sur le nouvel ordinateur depuis l’appli qui le stocke en ligne.
reason-manual-install = Cette appli doit venir du site de son éditeur. Téléchargez-la et installez-la depuis ce site.
reason-restart = Le nouvel ordinateur portable doit redémarrer pour terminer. Redémarrez-le quand vous le souhaitez.
reason-finish-by-hand = PCTwin ne peut pas faire cette étape à votre place. Suivez les étapes indiquées pour elle dans ce rapport.

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
mini-title-send-paused = Envoi en pause.
cancel-ack-hint = Cochez la case ci-dessus pour activer le bouton d’arrêt.
progress-state-placing = Mise en place
progress-state-existing = Déjà présent
outcome-stopped-title = Votre transfert a été arrêté.
outcome-stopped-body = Vous avez arrêté le transfert. Ce qui avait été copié et vérifié avant est gardé. Le reste n’a pas été copié.
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
qs-eyebrow-receiving = Réception sur cet ordinateur portable
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = De { $from } vers { $to }
qs-route-sender = Envoi · cet ordinateur portable
qs-route-receiver = Réception · même Wi-Fi
qs-tally =
    { NUMBER($count) ->
        [one] { $count } élément sélectionné · { $size }
        [many] { $count } d’éléments sélectionnés · { $size }
       *[other] { $count } éléments sélectionnés · { $size }
    }
qs-tally-badge = Envoi à proximité

## Q01 Quick Send home

qs-home-title = Quelques fichiers, envoyés directement.
qs-home-body = Envoyez quelques fichiers à un ordinateur portable à proximité.
qs-send-title = Envoyer des fichiers
qs-send-body = Choisissez des fichiers ou des dossiers sur cet ordinateur portable.
qs-receive-title = Recevoir des fichiers
qs-receive-body = Laissez un ordinateur portable à proximité envoyer des fichiers à celui-ci.
qs-history-title = Vos reçus
qs-history-body = Voyez où sont allés les fichiers reçus et ce qui n’a pas abouti.
qs-home-action = Choisir des fichiers
qs-receiving = Prêt à recevoir. Les autres ordinateurs portables à proximité voient celui-ci sous le nom { $label }.
qs-receive-stop = Arrêter la réception
qs-no-nearby = Aucun ordinateur portable trouvé à proximité. Ouvrez PCTwin sur l’autre ordinateur et gardez les deux sur le même Wi-Fi.

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
qs-online-only = Certains fichiers sont seulement en ligne. Rendez-les d’abord disponibles sur cet ordinateur portable, puis envoyez-les.
qs-protected = PCTwin ne peut pas lire { $name }. Autorisez l’accès, ou retirez-le de la liste.
qs-allow = Autoriser l’accès
qs-choose-continue = Vérifier ces fichiers

## Q03 Check what you are sharing

qs-check-title = Vérifiez ce que vous partagez.
qs-check-body = Relisez ces fichiers avant qu’ils quittent cet ordinateur portable.
qs-check-included = Inclus
qs-check-shared-note = L’autre ordinateur portable voit le nom, la taille et la date de ces fichiers.
qs-check-changed = { $name } a changé depuis votre choix. Vérifiez-le à nouveau.
qs-check-ack = J’ai vérifié ces fichiers
qs-check-ack-hint = Retirez tout ce qui est privé avant l’envoi.
qs-check-continue = Choisir l’autre ordinateur portable

## Q04 Who should receive it?

qs-who-title = Qui doit le recevoir ?
qs-who-body = Choisissez le nom affiché sur l’autre ordinateur portable.
qs-who-nearby = À proximité · même Wi-Fi.
qs-who-code = Utiliser un code d’association
qs-who-code-body = Saisissez le code affiché par l’expéditeur.
qs-who-same-label = Deux ordinateurs portables à proximité ont ce nom. Vérifiez le nom sur l’écran de l’autre ordinateur.
qs-who-busy = { $label } est occupé par un autre envoi. Attendez un instant, puis réessayez.
qs-who-note = L’autre ordinateur portable accepte les fichiers et choisit où ils vont.
qs-who-action = Afficher mon code d’envoi

## Q08 Incoming offer (receiving laptop)

qs-offer-title = Quelqu’un veut vous envoyer des fichiers.
qs-offer-body = Vérifiez ce qui est envoyé avant d’accepter.
qs-offer-note = PCTwin enregistre seulement les fichiers que vous acceptez. Il ne les ouvre et ne les lance jamais.
qs-offer-accept = Accepter et choisir un dossier
qs-offer-decline = Refuser ces fichiers
qs-offer-declined = Vous avez refusé. Rien n’a été enregistré sur cet ordinateur portable.
qs-offer-changed = L’expéditeur a modifié les fichiers. Vérifiez la nouvelle liste avant d’accepter.
qs-offer-gone = { $label } n’est plus disponible. Rien n’a été enregistré sur cet ordinateur portable.

## Q09 Where should the files land?

qs-land-title = Où enregistrer les fichiers ?
qs-land-body = Choisissez le dossier sur cet ordinateur portable.
qs-land-folder-label = Dossier de réception
qs-land-choose = Choisir un autre dossier
qs-land-space-title = Espace disponible
qs-land-space = Disponible : { $free } · nécessaire : { $needed }.
qs-land-status-fits = Suffisant
qs-land-status-no-space = Espace insuffisant
qs-land-no-space = Ce dossier a { $free } d’espace libre, mais ces fichiers ont besoin de { $needed }. Choisissez un autre dossier ou libérez de l’espace.
qs-land-unavailable = Ce dossier n’est pas disponible. Connectez son disque, ou choisissez un autre dossier.
qs-land-denied = PCTwin n’est pas autorisé à enregistrer dans ce dossier. Choisissez un autre dossier.
qs-land-note = Si un fichier du même nom s’y trouve déjà, vous choisissez ce qui se passe avant tout enregistrement.
qs-land-confirm = Utiliser ce dossier pour ces fichiers
qs-land-confirm-hint = Seuls les fichiers acceptés sont enregistrés ici.
qs-land-use = Utiliser ce dossier

## Q10 Keep your existing work safe

qs-conflict-title = Gardez votre travail en sécurité.
qs-conflict-body = Choisissez ce qui se passe quand un fichier du même nom est déjà là.
qs-conflict-col-file = Élément
qs-conflict-col-existing = Déjà présent
qs-conflict-col-choice = Choix
# $when is already formatted, for example "yesterday".
qs-conflict-existing = Modifié { $when }
qs-conflict-none = Aucune copie existante
qs-conflict-select =
    .aria-label = Que faire de { $name }
qs-keep-both = Garder les deux
qs-skip = Ignorer cet élément
qs-add-new = Ajouter le nouveau
qs-keep-both-note = Garder les deux laisse le fichier existant tel quel. Le nouveau est enregistré sous le nom { $newName }.
qs-no-conflicts = Aucun de ces fichiers n’est encore dans le dossier. Rien ne sera remplacé.
qs-conflict-target-changed = Si le fichier déjà présent change ou ne peut pas être ouvert, il attend que vous le vérifiiez à nouveau.
qs-conflict-approve = J’approuve ces choix
qs-conflict-continue = Confirmer ces choix

## Q11 Ready to send these files?

qs-approve-title = Envoyer ces fichiers maintenant ?
qs-approve-body = Vérifiez exactement ce qui est envoyé avant le début.
qs-approve-receiver-title = Autre ordinateur portable
qs-approve-receiver-waiting = En attente : l’autre ordinateur portable doit accepter et choisir un dossier.
qs-approve-receiver-ready = L’autre ordinateur portable a accepté, et a choisi un dossier et ce qu’il faut faire des fichiers déjà présents.
qs-approve-files-title = Fichiers
qs-approve-files-same = Ne modifiez pas ces fichiers avant la fin de l’envoi.
qs-approve-files-changed = Certains fichiers ont changé depuis votre vérification. Vérifiez-les à nouveau.
qs-approve-existing-title = Fichiers existants
qs-approve-existing-body = Garder les deux, comme choisi.
qs-status-waiting = En attente
qs-status-accepted = Approuvé
qs-status-unchanged = À vérifier
qs-status-changed = Modifié
qs-approve-ack = Envoyer ces fichiers
qs-approve-ack-hint = J’approuve ces fichiers et cet ordinateur portable.
qs-approve-start = Commencer l’envoi
qs-approve-cleared = Vous avez modifié les fichiers : { $label } doit donc les accepter à nouveau.
qs-approve-withdrawn = { $label } n’accepte plus cet envoi. Rien n’a été envoyé.

## Q12 Send progress

qs-sending-title = Vos fichiers sont en route.
qs-sending-body = Les copies vont directement d’un ordinateur portable à l’autre.
qs-sending-progress =
    .aria-label = Envoi des fichiers sélectionnés
qs-sending-current = Envoi en cours : { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = { $speed }, mesuré
qs-step-copying = Copie
qs-step-copying-body =
    { NUMBER($done) ->
        [one] { $done } élément sélectionné reçu.
        [many] { $done } d’éléments sélectionnés reçus.
       *[other] { $done } éléments sélectionnés reçus.
    }
qs-step-checking = Vérification
qs-step-checking-body = Chaque fichier est comparé à l’original.
qs-step-receipt = Reçu
qs-step-receipt-body = À la fin, voyez où sont allés les fichiers et ce qui n’a pas abouti.
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
qs-compact = Vue compacte
qs-stop = Arrêter cet envoi
qs-sending-complete-title = L’envoi est terminé
qs-sending-complete-body = Chaque fichier arrivé correspondait à son original. Voyez où ils ont été enregistrés.
qs-sending-receipt = Voir le reçu

## Q14 Handover receipt (both laptops)

qs-receipt-title = Voici votre reçu.
qs-receipt-body = Voyez exactement ce qui est arrivé et où.
qs-receipt-box-title = Vos fichiers sont arrivés
qs-receipt-box-body = Chaque copie a été comparée à l’original et correspondait.
qs-receipt-arrived =
    { NUMBER($count) ->
        [one] { $count } élément sélectionné reçu
        [many] { $count } d’éléments sélectionnés reçus
       *[other] { $count } éléments sélectionnés reçus
    }
qs-receipt-size = { $size } conformes à l’original
qs-receipt-saved-as = { $path }
qs-receipt-not-arrived = Ce dossier n’est pas entièrement arrivé et vérifié.
qs-receipt-status-verified = Vérifié
qs-receipt-status-unfinished = Inachevé
qs-receipt-not-completed = Non terminé.
qs-receipt-stopped-row = Reçu et vérifié avant l’arrêt.
qs-receipt-partial =
    { NUMBER($count) ->
        [one] { $count } élément demande encore votre attention.
        [many] { $count } d’éléments demandent encore votre attention.
       *[other] { $count } éléments demandent encore votre attention.
    }
qs-receipt-stopped = Cet envoi a été arrêté. Les fichiers arrivés en entier restent dans le dossier choisi.
qs-receipt-not-opened = PCTwin n’a ouvert ni lancé aucun fichier reçu.
qs-receipt-ack = Confirmer ce reçu
# $when is already formatted, for example "today at 14:05".
qs-receipt-acked = Reçu confirmé { $when }.
qs-receipt-ack-note = Confirmer ce reçu ne signifie pas que les fichiers ont été ouverts ou utilisés.
qs-receipt-offline = { $label } est hors ligne et n’a donc pas encore vu ce reçu. Le reçu est enregistré sur cet ordinateur portable.
qs-receipt-missing = Envoyer ce qui manque
qs-receipt-missing-body = Vérifiez seulement les fichiers qui ne sont pas arrivés.
qs-receipt-history = Tous les reçus
qs-receipt-done = Retour à Envoi rapide

## Q19 Finish the missing part

qs-missing-title = Terminez la partie manquante.
qs-missing-body = Utilisez le reçu précédent pour voir ce qui n’est pas arrivé.
qs-missing-status-there = Arrivé
qs-missing-status-missing = Action nécessaire
qs-missing-there-body = Il est arrivé et correspondait à l’original.
qs-missing-missing-body = Vérifiez les fichiers manquants ou inachevés.
qs-missing-note = Les fichiers arrivés sont gardés. Les fichiers nouveaux ou modifiés doivent être approuvés à nouveau.
qs-missing-ack = Envoyer les fichiers manquants
qs-missing-ack-hint = PCTwin vérifie à nouveau les fichiers, et l’autre ordinateur portable les accepte à nouveau.
qs-missing-complete = Tout est déjà arrivé. Il ne reste rien à envoyer.
qs-missing-unavailable = Le reçu précédent est introuvable, PCTwin ne peut donc pas savoir ce qui manque. Lancez plutôt un nouvel envoi.
qs-missing-continue = Vérifier ce qui sera envoyé

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
qs-stop-confirm = Arrêter et voir le reçu
qs-stop-keep = Continuer l’envoi
qs-decline-title = Refuser ces fichiers ?
qs-decline-body = Rien n’est enregistré sur cet ordinateur portable, et l’expéditeur est informé de votre refus.
qs-decline-confirm = Refuser
qs-already-finished = Cet envoi est déjà terminé. Consultez le reçu pour voir ce qui est arrivé.

## F08 Your moves, kept on this laptop (history)

history-title = Vos transferts, gardés sur cet ordinateur portable.
history-body = Rouvrez un rapport, ou reprenez un transfert enregistré.
history-row-move = { $from } → { $to }
history-row-sent = Envoyé à { $label }
history-row-received = Reçu de { $label }
history-status-finished = Terminé
history-status-steps-left =
    { NUMBER($count) ->
        [one] Terminé, { $count } étape pour vous
        [many] Terminé, { $count } d’étapes pour vous
       *[other] Terminé, { $count } étapes pour vous
    }
history-status-paused = En pause
history-status-stopped = Arrêté en toute sécurité
history-status-partial = Arrivé en partie
history-open-report = Rapport
history-open-receipt = Reçu
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
qs-check-missing = { $name } n’est plus là. Revenez en arrière et retirez-le, ou remettez-le en place.
qs-check-unreadable = PCTwin ne peut plus lire { $name }. Autorisez l’accès, ou revenez en arrière et retirez-le.
qs-check-show-all =
    { NUMBER($count) ->
        [one] Afficher le fichier
        [many] Afficher les { $count } fichiers
       *[other] Afficher les { $count } fichiers
    }
qs-check-change = Modifier ce que j’envoie
qs-land-none = Aucun dossier choisi.
qs-land-space-unknown = PCTwin n’a pas pu mesurer l’espace libre ici. Ces fichiers ont besoin de { $needed }.
qs-conflict-existing-unknown = Déjà présent
qs-conflict-choice-open = Pas encore choisi
qs-conflict-why = Choisissez ce qui arrive à chaque fichier, puis cochez la case, pour continuer.
qs-approve-folder-title = Dossier de réception
qs-status-recipient-choice = Choix du destinataire
qs-sending-card = Envoi des fichiers sélectionnés
qs-progress-of = sur { $total }
qs-receipt-partial-title = Tout n’est pas arrivé
qs-receipt-metric-items =
    { NUMBER($count) ->
        [one] élément sélectionné reçu
        [many] d’éléments sélectionnés reçus
       *[other] éléments sélectionnés reçus
    }
qs-receipt-metric-size = vérifiés par rapport à l’original
qs-receipt-sent-as = Envoyé sous le nom { $path }
qs-receipt-renamed = Un fichier de ce nom était déjà là, celui-ci a donc reçu un nouveau nom.
qs-receipt-chip-already = Déjà présent
qs-receipt-already = Un fichier identique était déjà là ({ $path }), il n’a donc pas été renvoyé.
qs-receipt-chip-declined = Non accepté
qs-receipt-declined-send = L’autre ordinateur portable n’a pas accepté ce fichier, il n’a donc pas été envoyé.
qs-receipt-declined-receive = Vous n’avez pas accepté ce fichier, il n’a donc pas été enregistré.
qs-receipt-more = Plus de détails
qs-receipt-empty = Rien n’a été envoyé, il n’y a donc rien à afficher.
qs-receipt-already-count =
    { NUMBER($count) ->
        [one] { $count } élément était déjà là, identique
        [many] { $count } d’éléments étaient déjà là, identiques
       *[other] { $count } éléments étaient déjà là, identiques
    }
qs-missing-new-send = Lancer un nouvel envoi
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
