# PCTwin screens, Hindi (hi). Machine-translated ahead of time from ../en/main.ftl;
# a string missing here falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = PCTwin होम
language-label = भाषा
motion-label = एनिमेशन
motion-system = एनिमेशन: सिस्टम के अनुसार
motion-reduced = एनिमेशन: कम
motion-off = एनिमेशन: बंद

# The opening (Wordplay); its tagline is the home headline.
opening-label = PCTwin खुल रहा है
opening-status = PCTwin खुल रहा है।
opening-skip = परिचय छोड़ें
opening-tagline = फ़ाइलें भेजें, या सब कुछ ले जाएँ।

footer-free = मुफ़्त। किसी अकाउंट की ज़रूरत नहीं।
footer-help = मदद

## F01 Home

welcome-title-line1 = फ़ाइलें भेजें,
welcome-title-line2 = या सब कुछ ले जाएँ।
welcome-body = आस-पास के किसी लैपटॉप पर कुछ फ़ाइलें भेजें, या अपनी फ़ाइलें, ऐप्स और सेटिंग्स अपने नए लैपटॉप पर ले जाएँ।
welcome-start = पूरा स्थानांतरण शुरू करें

## F01 Home: the cards, My history, a move to resume, files waiting

home-quick-send-body = फ़ाइलें भेजें या पाएँ
home-move-body = अपना सेटअप ले जाएँ
home-restore-body = सहेजे गए सेटअप से दोबारा बनाएँ
home-rescue-body = ऐसे लैपटॉप से फ़ाइलें कॉपी करें जो चालू नहीं होता
home-history = मेरा इतिहास
home-history-body = आपके स्थानांतरण और उनकी रिपोर्ट
name-title = इस लैपटॉप को नाम दें
name-body = पास के दूसरे लैपटॉप इसे इसी नाम से ढूँढेंगे। आप इसे बाद में, जोड़ते समय बदल सकते हैं।
name-use = यही नाम रखें
home-update-title = नया संस्करण उपलब्ध है
home-update-action = अभी अपडेट करें
home-updating = अपडेट हो रहा है। PCTwin बंद होकर अपने आप फिर से खुलेगा।
home-update-failed = अपडेट इंस्टॉल नहीं हो सका। PCTwin पहले की तरह काम करता है; अगली बार अपडेट फिर से दिखेगा।
home-update-busy = पहले अपना स्थानांतरण पूरा करें। फिर PCTwin अपडेट करें।
home-resume-body = आपकी प्रगति सहेजी गई है।
home-resume-action = फिर शुरू करें
    .aria-label = अपना स्थानांतरण फिर से शुरू करें
home-waiting =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल { $to } के लिए रुकी है
       *[other] { $count } फ़ाइलें { $to } के लिए रुकी हैं
    }

## Header: this laptop, the language and motion control, Help

header-this-laptop = आपका लैपटॉप

## F06 Old or new laptop

role-back = वापस
role-eyebrow = पूरा स्थानांतरण शुरू करें
role-title = यह कौन-सा लैपटॉप है?
role-body = दोनों लैपटॉप पर PCTwin खोलें। फिर बताएं कि यह कौन-सा है।
role-old-title = यह मेरा पुराना लैपटॉप है
role-old-body = मेरी फ़ाइलें, ऐप्स और सेटिंग्स भेजें।
role-new-title = यह मेरा नया लैपटॉप है
role-new-body = दूसरे लैपटॉप से मेरा सेटअप प्राप्त करें।
role-note = योजना देखकर मंज़ूर करने तक कुछ भी नहीं जाता। स्थानांतरण के दौरान आपके पुराने लैपटॉप में कुछ नहीं बदलता।

## Frame parts (footer note, Back, steps)

footer-note-move = निजी स्थानांतरण मुफ़्त
footer-note-send = आस-पास भेजना मुफ़्त
frame-back = वापस
# Read out by screen readers only; the step shapes carry the meaning on screen.
frame-step-done = पूरा
frame-step-current = अभी
frame-step-todo = आगे
frame-step-position = { $total } में से चरण { $number }

## F01 Unfinished move

resume-open = अपना स्थानांतरण फिर से शुरू करें

## C01 How will you connect?

connect-eyebrow = आपका स्थानांतरण
connect-title = कनेक्ट कैसे करें?
connect-body = कोई भी तरीका चुनें, चरण वही रहते हैं।
connect-wifi-title = Wi-Fi
connect-wifi-body = दोनों लैपटॉप एक ही Wi-Fi पर।
connect-wifi-status = उपलब्ध
connect-later = नेटवर्क केबल, PCTwin ब्रिज केबल और बाहरी ड्राइव से स्थानांतरण बाद के किसी वर्शन में आएंगे।
connect-usb-note = साधारण USB केबल से दो लैपटॉप को स्थानांतरण के लिए कनेक्ट नहीं किया जा सकता।
connect-use-wifi = Wi-Fi इस्तेमाल करें
# Added: sizes and speeds as measured, shown the same way on every connection screen.
connect-unit-mbps = { NUMBER($value, maximumFractionDigits: 0) } MB/s
connect-unit-gb = { NUMBER($value, maximumFractionDigits: 1) } GB

## C02 Put both laptops together

connect-prep-title = दोनों लैपटॉप पास-पास रखें
connect-prep-body = दोनों को एक ही Wi-Fi से कनेक्ट करें, फिर हर एक पर PCTwin खोलें।
connect-prep-step-1-title = एक ही Wi-Fi से कनेक्ट करें
connect-prep-step-1-body = फ़ाइलें कॉपी करने के लिए इंटरनेट की ज़रूरत नहीं है।
# $network is the Wi-Fi name this laptop is on.
connect-prep-step-1-network = नेटवर्क: { $network }।
connect-prep-step-2-title = आस-पास के लैपटॉप खोजने की अनुमति दें
connect-prep-step-2-body = अगर Windows या macOS अनुमति मांगे, तो “अनुमति दें” चुनें।
connect-prep-step-3-title = हर लैपटॉप पर उसकी भूमिका चुनें
connect-prep-step-3-body = एक भेजता है। दूसरा प्राप्त करता है।
connect-prep-note = गेस्ट या सार्वजनिक Wi-Fi पर लैपटॉप शायद एक-दूसरे को न देख पाएं।
connect-prep-continue = लैपटॉप जोड़ने के लिए आगे बढ़ें

## C06 Meet your other laptop

peer-eyebrow = आपका स्थानांतरण
peer-title = दोनों लैपटॉप जाँचें
peer-body = दोनों लैपटॉप कनेक्ट हो गए हैं और तैयार हैं।
peer-sending = भेज रहा है
peer-receiving = प्राप्त कर रहा है
peer-old = पुराना लैपटॉप
peer-new = नया लैपटॉप
peer-version-label = PCTwin
# $version is the PCTwin version on that laptop, for example "1.0.3".
peer-version = { $version }
peer-status-update = अपडेट ज़रूरी
peer-status-unsupported = इंतज़ार
peer-ready-note = दोनों लैपटॉप जुड़ने के बाद ही PCTwin आपकी फ़ाइलों की सूची बनाता है। अगर वर्शन अलग हैं, तो दोनों पर PCTwin अपडेट करें।
peer-version-differs = इन लैपटॉप पर PCTwin के अलग-अलग वर्शन हैं। आगे बढ़ने के लिए दोनों पर PCTwin अपडेट करें।
peer-version-old-body = अलग वर्शन: { $version }।
peer-version-new-body = दोनों लैपटॉप पर एक ही वर्शन चाहिए।
peer-update-title = PCTwin अपडेट करें
peer-update-body = दोनों लैपटॉप पर एक ही वर्शन रखें।
# $from and $to are system names, for example "Windows 11" and "macOS". The mockup sentence does not use them.
peer-direction-unsupported = PCTwin का यह वर्शन अभी इन दो सिस्टम के बीच स्थानांतरण नहीं कर सकता।
peer-continue = कनेक्शन की जाँच करें

## C07 Let’s check the connection

check-eyebrow = आपका स्थानांतरण
check-title = कनेक्शन जाँचें
check-body = स्थानांतरण की योजना बनाने और समय का अनुमान लगाने के लिए PCTwin गति, स्टोरेज और बिजली की जाँच करता है।
check-measuring = कनेक्शन मापा जा रहा है…
# The value next to it is already formatted, for example "62 MB/s".
check-speed-label = कनेक्शन की गति
check-space-label = नए लैपटॉप पर उपलब्ध
check-link-weak = कनेक्शन कमज़ोर है, इसलिए स्थानांतरण धीमा हो सकता है। लैपटॉप को Wi-Fi राउटर के पास ले जाने से मदद मिल सकती है।
check-power-title = बिजली
check-power-both = दोनों लैपटॉप प्लग इन हैं।
check-power-battery = { $label } बैटरी पर चल रहा है। इसे प्लग इन करें, ताकि लंबा स्थानांतरण बीच में न रुके।
check-space-title = स्टोरेज
check-space-body = नए लैपटॉप पर { $free } स्टोरेज उपलब्ध है। कुछ भी जाने से पहले दिखाया जाएगा कि आपकी चुनी चीज़ें इसमें आएंगी या नहीं।
check-internet-title = इंटरनेट
check-internet-on = कनेक्ट है। फ़ाइलें कॉपी करने के लिए इंटरनेट की ज़रूरत नहीं है।
check-internet-off = कनेक्ट नहीं है। कोई बात नहीं: फ़ाइलें कॉपी करने के लिए इंटरनेट की ज़रूरत नहीं है।
check-status-ready = तैयार
check-status-plug-in = प्लग इन करें
check-status-connected = उपलब्ध
check-status-not-needed = ज़रूरी नहीं
# Added: what is said when something could not be read, so it is never shown as fine.
check-status-unknown = पता नहीं
check-speed-unknown = PCTwin इस बार कनेक्शन नहीं माप सका। फ़ाइलें फिर भी जाएँगी; बस समय का अनुमान नहीं मिलेगा।
check-space-unknown = PCTwin नए लैपटॉप पर उपलब्ध स्टोरेज नहीं पढ़ सका।
check-power-unknown = PCTwin पता नहीं लगा सका कि { $label } प्लग इन है या नहीं।
check-internet-unknown = PCTwin पता नहीं लगा सका कि यह लैपटॉप ऑनलाइन है या नहीं। कोई बात नहीं: फ़ाइलें कॉपी करने के लिए इंटरनेट की ज़रूरत नहीं है।
check-again = फिर से जाँच करें
check-estimate-note = समय का अनुमान कोई पक्का वादा नहीं है।
check-continue = चुनें कि क्या भेजना है
check-continue-new = आगे बढ़ें
# Added 23 October 2026: the short checklist both laptops show before the move.
check-before-title = स्थानांतरण शुरू होने से पहले
check-before-lids-title = दोनों लैपटॉप के ढक्कन खुले रखें
check-before-lids-body = ढक्कन बंद करने से लैपटॉप स्लीप में जा सकता है, जिससे स्थानांतरण रुक जाता है।
check-before-near-title = Wi-Fi राउटर के पास रहें
check-before-near-body = जब तक आपकी चीज़ें जा रही हैं, दोनों लैपटॉप उसके पास रखें।

## C09 Waiting for your other laptop

reconnect-title = आपके दूसरे लैपटॉप का इंतज़ार है
reconnect-body = अब तक कॉपी और जाँची गई हर चीज़ रखी गई है।
# Added: the title of the saved-progress card (the mockup shows Move paused by connection).
reconnect-card-title = कनेक्शन के कारण स्थानांतरण रुका हुआ है
reconnect-progress =
    .aria-label = कनेक्शन के कारण स्थानांतरण रुका हुआ है
# $done is an already formatted size, for example "81.6 GB". $total is passed but not shown.
reconnect-saved = अब तक { $done } कॉपी और जाँचा गया। बाकी अभी आना है।
# $ago is already formatted, for example "45 seconds ago".
reconnect-last-seen = आख़िरी बार { $ago } दिखा।
reconnect-status-offline = ऑफ़लाइन
reconnect-status-trying = फिर से कनेक्ट हो रहा है
reconnect-status-pending = इंतज़ार
reconnect-trying = अपने-आप फिर से कनेक्ट करने की कोशिश हो रही है… दोनों लैपटॉप चालू रखें, स्लीप में न जाने दें, और एक ही Wi-Fi पर रखें।
reconnect-expired = अपने-आप फिर से कनेक्ट होने का समय निकल गया। फिर से जोड़ें, और PCTwin जाँच लेगा कि क्या पहले ही कॉपी हो चुका है।
reconnect-unavailable = { $label } तक पहुँचा नहीं जा सकता। जाँचें कि वह चालू है, स्लीप में नहीं है, उस पर PCTwin खुला है और वह इसी Wi-Fi पर है।
reconnect-try-now = फिर से कनेक्ट करें
reconnect-pair-again = फिर से जोड़ें
reconnect-nothing-new = कुछ समय से कुछ भी नया नहीं आया है। PCTwin अपने-आप कोशिश करता रहेगा, या आप स्थानांतरण रोक सकते हैं। अब तक कॉपी और जाँची गई हर चीज़ बनी रहेगी।
reconnect-details = सुरक्षित तरीके से फिर से कनेक्ट होना
reconnect-details-body = सिर्फ़ ये दो लैपटॉप फिर से कनेक्ट हो सकते हैं, और हर बार वे एक नया निजी कनेक्शन इस्तेमाल करते हैं।

## C10 The connection has ended (dialog)

ended-title = कनेक्शन ख़त्म हो गया है
ended-body = आपका स्थानांतरण सहेजा गया है। पुराना कोड अब काम नहीं करता।
ended-kept-title = रखा गया
ended-kept-body = आपके स्थानांतरण का रिकॉर्ड और कॉपी व जाँची गई हर चीज़।
ended-not-kept-title = नहीं रखा गया
ended-not-kept-body = फिर से जोड़े बिना आगे बढ़ने की अनुमति।
ended-pair-again = फिर से जोड़ें
ended-pair-again-body = फिर से कनेक्ट करें, और PCTwin जाँच लेगा कि क्या पहले ही कॉपी हो चुका है।
ended-see-moved = सहेजा गया स्थानांतरण खोलें
ended-see-moved-body = देखें कि अब तक क्या कॉपी और जाँचा जा चुका है।
ended-home = होम पर लौटें

## Pairing, shared

pair-back = वापस
pair-try-again = फिर से कोशिश करें
pair-more-details = ज़्यादा जानकारी
pair-heads-up-title = आपका लैपटॉप आपसे एक सवाल पूछ सकता है
pair-heads-up-local-network = आपका Mac पूछेगा कि क्या PCTwin आपके लोकल नेटवर्क पर डिवाइस ढूँढ सकता है। “अनुमति दें” चुनें, ताकि आपके दोनों लैपटॉप एक-दूसरे को ढूँढ सकें।
pair-heads-up-firewall = Windows पूछ सकता है कि क्या PCTwin कनेक्शन स्वीकार कर सकता है। “अनुमति दें” चुनें, ताकि आपका नया लैपटॉप इस लैपटॉप तक पहुँच सके।
pair-heads-up-continue = जारी रखें

## C03 Old laptop: show the code

pair-old-eyebrow = पुराना लैपटॉप
pair-old-title = अपने लैपटॉप जोड़ें
pair-old-body = अपने नए लैपटॉप पर नीचे दिया गया कोड डालें।
pair-old-next = इसके बाद, दोनों लैपटॉप पर दिखने वाला एक नंबर मिलाना होगा।
pair-old-network = दोनों लैपटॉप एक ही Wi-Fi पर रखें।
pair-old-waiting = आपके नए लैपटॉप का इंतज़ार है…
pair-old-starting = कोड तैयार हो रहा है…
pair-old-label-title = इस लैपटॉप का नाम
pair-old-label-hint = आपका नया लैपटॉप इसे इसी नाम से ढूंढेगा।
pair-old-shuffle = दूसरा नाम चुनें
pair-old-rename = नाम लिखें
pair-old-rename-label = इस लैपटॉप का नाम
pair-old-rename-note = इस Wi-Fi पर कोई भी यह नाम देख सकता है।
pair-old-rename-save = यह नाम इस्तेमाल करें
pair-old-rename-cancel = रद्द करें
pair-old-code-meta =
    { $seconds ->
        [one] हर मिनट बदलता है। नया कोड { $seconds } सेकंड में।
       *[other] हर मिनट बदलता है। नया कोड { $seconds } सेकंड में।
    }
pair-old-steps-title = अपने नए लैपटॉप पर
pair-old-step-1 = PCTwin खोलें और “मेरा नया लैपटॉप” चुनें।
pair-old-step-2 = अगर कोई सूची दिखे, तो इस लैपटॉप का नाम चुनें।
pair-old-step-3 = यहाँ दिखाया गया छह अंकों का कोड लिखें।
pair-old-wait-title = नए कोड की प्रतीक्षा करें
pair-old-wait-body =
    { $seconds ->
        [one] वह कोशिश काम नहीं आई। नया कोड { $seconds } सेकंड में दिखेगा।
       *[other] वह कोशिश काम नहीं आई। नया कोड { $seconds } सेकंड में दिखेगा।
    }
pair-old-locked-title = इस नेटवर्क पर किसी ने गलत कोड आज़माए
pair-old-locked-body = पाँच गलत कोड के बाद, इस लैपटॉप को सुरक्षित रखने के लिए जोड़ना बंद कर दिया गया। कुछ भी शेयर नहीं हुआ। अगर यह आप थे, तो नया कोड पाने के लिए “फिर से शुरू करें” चुनें।
pair-old-locked-hotspot = अगर आपको नहीं पता कि यह कौन था, तो दोनों लैपटॉप को अपने फ़ोन के हॉटस्पॉट से कनेक्ट करें, फिर “फिर से शुरू करें” चुनें।
pair-old-start-again = फिर से शुरू करें
pair-old-timed-out = समय पर कोई नंबर नहीं चुना गया, इसलिए यह नया कोड है।
pair-old-declined = नंबर मेल नहीं खाए, इसलिए कुछ भी शेयर नहीं हुआ। यह नया कोड है।
pair-old-left = नंबर चुने जाने से पहले नए लैपटॉप ने जोड़ना बंद कर दिया, इसलिए कुछ भी शेयर नहीं हुआ। यह नया कोड है।
pair-network-title = अपना Wi-Fi चुनें
pair-network-hint = वह नेटवर्क चुनें जिस पर आपका नया लैपटॉप है।

## C05 Old laptop: match the number

pair-choose-eyebrow = कनेक्शन की पुष्टि करें
pair-choose-title = मिलता हुआ नंबर चुनें
pair-choose-body = अपने नए लैपटॉप को देखें। जो नंबर वह दिखाता है, वही चुनें।
pair-choose-number = नंबर { $number }
pair-choose-confirm = इन लैपटॉप की पुष्टि करें
pair-choose-renewed = नए नंबर दिखाए गए हैं। वह नंबर चुनें जो आपके नए लैपटॉप पर है।
pair-choose-none = इनमें से कोई मेल नहीं खाता
pair-choose-note = अगर नया लैपटॉप कोई नंबर नहीं दिखाता, या इनमें से कोई उससे मेल नहीं खाता, तो “इनमें से कोई मेल नहीं खाता” चुनें।
pair-choose-safe = अभी तक कुछ भी शेयर नहीं हुआ है।
pair-choose-checking = जाँच हो रही है…

## C04 New laptop: find the old laptop and enter the code

pair-new-eyebrow = नया लैपटॉप
pair-new-searching-title = आपका पुराना लैपटॉप खोजा जा रहा है…
pair-new-searching-body = अपने पुराने लैपटॉप पर PCTwin खुला रखें, और उसे उसी Wi-Fi पर रखें जिस पर यह लैपटॉप है।
pair-new-search-failed-title = आपका पुराना लैपटॉप ढूँढा नहीं जा सका
pair-new-list-title = इनमें से आपका पुराना लैपटॉप कौन-सा है?
pair-new-list-body = वह नाम चुनें जो आपका पुराना लैपटॉप अपनी स्क्रीन पर दिखाता है।
pair-new-same-label = आस-पास के एक और लैपटॉप का भी यही नाम है। पुराने लैपटॉप की स्क्रीन देखें।
pair-new-needs-update = इस लैपटॉप पर PCTwin का पुराना संस्करण है। उस पर PCTwin अपडेट करें, फिर दोबारा खोजें।
pair-new-none-title = कोई पुराना लैपटॉप नहीं मिला
pair-new-none-body = जाँचें कि आपके पुराने लैपटॉप पर PCTwin खुला है और कोड दिखा रहा है, और दोनों लैपटॉप एक ही Wi-Fi पर हैं। फिर से खोजें।
pair-new-search-again = फिर से खोजें
pair-old-same-role = पास का एक और लैपटॉप भी पुराने लैपटॉप के रूप में सेट है। दोनों में से एक नया लैपटॉप होना चाहिए।
pair-old-switch = यह नया लैपटॉप है
pair-new-none-same-role = क्या दूसरे लैपटॉप ने भी नया लैपटॉप चुना है? तो उस पर पुराना लैपटॉप चुनें, या इसे बदलें।
pair-new-switch = यह पुराना लैपटॉप है
pair-new-code-title = कोड डालें
pair-new-code-body = { $label } पर दिखाया गया कोड लिखें।
pair-new-code-label = छह अंकों का कोड
pair-new-code-hint = इसे अपने पुराने लैपटॉप की स्क्रीन से पढ़ें। यह हर मिनट बदलता है।
pair-new-connect = कनेक्ट करें
pair-new-connecting = कनेक्ट हो रहा है…
pair-new-other-laptop = कोई दूसरा लैपटॉप चुनें
pair-new-number-eyebrow = कनेक्शन की पुष्टि करें
pair-new-number-title = अपने पुराने लैपटॉप पर यह नंबर चुनें
pair-new-number-tile = आपका नंबर
pair-new-number-label = आपका नंबर { $number } है
pair-new-number-body = आपका पुराना लैपटॉप तीन नंबर दिखाता है। वहाँ यही नंबर चुनें।
pair-new-number-waiting = आपके पुराने लैपटॉप का इंतज़ार है…

## Paired (both laptops)

pair-done-eyebrow = जोड़े गए
pair-done-title = आपके लैपटॉप जोड़ दिए गए हैं
pair-done-body = आपके दोनों लैपटॉप जुड़ गए हैं। अब पुराना लैपटॉप आपकी फ़ाइलें ढूँढेगा, और नए लैपटॉप पर आप मंज़ूर करेंगे कि क्या जाएगा। आपकी मंज़ूरी से पहले कुछ नहीं जाता।
pair-done-continue = आगे बढ़ें

## Pairing problems (each says what happened, then what to do)

pairing-code-invalid = कोड में छह अंक होने चाहिए। पुराने लैपटॉप की स्क्रीन देखें और कोड फिर से लिखें।
pairing-code-wrong = कोड मेल नहीं खाया। पुराने लैपटॉप की स्क्रीन देखें और फिर से कोशिश करें।
pairing-code-expired = कोड बदल गया है। पुराने लैपटॉप की स्क्रीन से नया कोड लिखें।
pairing-busy = पुराना लैपटॉप अभी किसी दूसरे लैपटॉप से जुड़ रहा है। थोड़ा इंतज़ार करें, और फिर से कोशिश करें।
pairing-wait = पुराना लैपटॉप नया कोड तैयार कर रहा है। उसका इंतज़ार करें, और फिर नया कोड लिखें।
pairing-locked = बहुत ज़्यादा गलत कोशिशों के बाद जोड़ना बंद हो गया। पुराने लैपटॉप पर “फिर से शुरू करें” चुनें।
pairing-unreachable = यह लैपटॉप पुराने लैपटॉप तक नहीं पहुँच सका। जाँचें कि दोनों एक ही Wi-Fi पर हैं, और फिर से कोशिश करें।
pairing-wrong-number = वह नंबर नए लैपटॉप के नंबर से मेल नहीं खाया। कुछ भी शेयर नहीं हुआ। फिर से कोशिश करने के लिए नए लैपटॉप पर नया कोड लिखें।
pairing-no-match = पुराने लैपटॉप पर नंबर मेल नहीं खाए। कुछ भी शेयर नहीं हुआ। पुराने लैपटॉप की स्क्रीन से नया कोड लिखें।
pairing-timed-out = समय पर कोई नंबर नहीं चुना गया। पुराने लैपटॉप पर दिख रहे नए कोड से फिर से कोशिश करें।
pairing-connection-lost = लैपटॉप के बीच कनेक्शन टूट गया। कुछ भी शेयर नहीं हुआ। फिर से कोशिश करें।
pairing-connection-lost-new = पुराने लैपटॉप से कनेक्शन टूट गया। कुछ भी शेयर नहीं हुआ।
pairing-different-version = दोनों लैपटॉप पर PCTwin के अलग-अलग वर्शन हैं। दोनों पर PCTwin अपडेट करें, और फिर से कोशिश करें।
pairing-name-invalid = यह नाम इस्तेमाल नहीं हो सकता। अक्षरों और अंकों वाला कोई छोटा नाम आज़माएं।
pairing-no-network = यह लैपटॉप किसी Wi-Fi या तार वाले नेटवर्क पर नहीं है। इसे कनेक्ट करें, और फिर से कोशिश करें।
pairing-choose-network = यह लैपटॉप एक से ज़्यादा नेटवर्क पर है। वह चुनें जिस पर आपका नया लैपटॉप है:
pairing-local-network-refused = हो सकता है आपका Mac, PCTwin को इस नेटवर्क पर लैपटॉप ढूँढने से रोक रहा हो। अगर आपके Mac ने अभी पूछा था, तो “अनुमति दें” चुनें और फिर से कोशिश करें। नहीं तो सिस्टम सेटिंग्ज़ खोलें, “गोपनीयता और सुरक्षा” चुनें, फिर “लोकल नेटवर्क” चुनें, PCTwin चालू करें, और फिर से कोशिश करें।
pairing-local-network-refused-details = VPN, फ़ायरवॉल या फ़िल्टर ऐप भी लैपटॉप को एक-दूसरे को ढूँढने से रोक सकता है। अगर इनमें से कोई चालू है, तो जोड़ते समय उसे बंद करें, फिर से कोशिश करें।
pairing-failed = इस बार जोड़ना काम नहीं आया। कुछ भी शेयर नहीं हुआ। फिर से कोशिश करें।
pairing-failed-new = इस बार जोड़ना काम नहीं किया। कुछ भी साझा नहीं हुआ। पुराने लैपटॉप की स्क्रीन से नया कोड लिखें।

## M06 Match people to their profiles (Who’s moving?)

people-eyebrow = आपका स्थानांतरण
people-title = लोगों को उनके अकाउंट से मिलाएं
people-body = चुनें कि नए लैपटॉप पर हर व्यक्ति की फ़ाइलें कहाँ जाएं।
people-col-old = पुराना लैपटॉप
people-col-new = नया लैपटॉप
people-col-status = स्थिति
people-you = आप
# $name is the person's name on the old laptop. $account is an account name on the new laptop.
people-select =
    .aria-label = { $name } के लिए नए लैपटॉप पर अकाउंट
people-option-existing = { $account } / नया लैपटॉप
people-option-new = नया अकाउंट बनाएं
people-option-leave-out = छोड़ दें
people-option-choose = चुनें…
people-new-account-hint = अकाउंट बनाने से पहले नया लैपटॉप अनुमति मांगता है।
people-suggested = नाम मिलते हैं, इसलिए यह सुझाया गया है। जाँच लें कि यह वही व्यक्ति है।
# $account is an account name on the new laptop.
people-suggested-use = { $account } इस्तेमाल करें
people-status-ready = पुष्टि हो गई
people-status-waiting = उनकी मंज़ूरी चाहिए
people-status-left-out = छोड़ा गया
people-status-choose = आपको चुनना है
people-status-changed = आपने बदला
people-waiting-body = उनकी फ़ाइलें जाने से पहले { $name } को पुराने लैपटॉप पर साइन इन करके मंज़ूरी देनी होगी। तब तक उनकी फ़ाइलों को छुआ नहीं जाता।
people-just-me = पुराने लैपटॉप पर सिर्फ़ आपका अपना अकाउंट मिला।
people-changed-note = आपने बदला है कि किसी की फ़ाइलें कहाँ जाएंगी। आगे बढ़ने से पहले इसे जाँच लें।
people-shared-title = शेयर किए गए आइटम
people-shared-body = शेयर किए गए फ़ोल्डर और सार्वजनिक दस्तावेज़, जो पुराने लैपटॉप पर सभी के हैं।
people-shared-select =
    .aria-label = शेयर किए गए आइटम कहाँ जाएं
people-shared-option-shared = नए लैपटॉप पर शेयर किए गए फ़ोल्डर
people-shared-option-person = { $name } की फ़ाइलों के साथ
people-shared-option-leave-out = छोड़ दें
people-note = PCTwin कभी किसी का पासवर्ड नहीं बदलता, और न ही किसी दूसरे व्यक्ति के साइन-इन को दरकिनार करता है।
people-list-label = पुराने लैपटॉप पर मौजूद लोग
people-todo =
    { $count ->
        [0] जो भी जा सकता है, उन सभी की जगह तय है।
        [one] { $count } व्यक्ति के लिए अभी चुनना बाकी है।
       *[other] { $count } लोगों के लिए अभी चुनना बाकी है।
    }
people-waiting-count =
    { $count ->
        [one] { $count } व्यक्ति की मंज़ूरी का इंतज़ार है। आगे बढ़ना ठीक है; मंज़ूरी मिलने तक उनकी फ़ाइलों को छुआ नहीं जाता।
       *[other] { $count } लोगों की मंज़ूरी का इंतज़ार है। आगे बढ़ना ठीक है; मंज़ूरी मिलने तक उनकी फ़ाइलों को छुआ नहीं जाता।
    }
people-continue = आगे बढ़ें

## M26 Before you leave this laptop (things to do on the old laptop)

before-eyebrow = आपका स्थानांतरण
before-title = इस लैपटॉप को छोड़ने से पहले
before-body = जब तक पुराना लैपटॉप आपके पास है, ये चीज़ें जाँच लें।
before-licences-title = ऐप लाइसेंस
before-licences-body = कुछ ऐप्स का लाइसेंस आपको ख़ुद ले जाना होगा।
before-passwords-title = अकाउंट रिकवरी
before-passwords-body = पक्का करें कि आपके अकाउंट और लॉक की गई ड्राइव अब भी खोले जा सकते हैं।
before-codes-title = साइन-इन कोड पास रखें
before-codes-body = अगर कोई अकाउंट आपको कोड भेजता है या आपकी पहचान की पुष्टि के लिए किसी ऐप का इस्तेमाल करता है, तो वह फ़ोन या ऐप पास रखें।
before-online-title = बिना सहेजा काम
before-online-body = कोई भी ड्राफ़्ट सहेज लें, और जाँचें कि ऑनलाइन रखी फ़ाइलें पूरी तरह अपडेट हैं।
before-done =
    .aria-label = मैंने यह कर लिया है: { $item }
before-note = यहाँ पासवर्ड या रिकवरी कुंजी न लिखें। यह सूची पूरी करने का मतलब यह नहीं कि पुराने लैपटॉप को मिटाना सुरक्षित है।
before-empty = स्थानांतरण से पहले आपके पुराने लैपटॉप पर कुछ भी करने की ज़रूरत नहीं है।
before-continue = स्थानांतरण की योजना पर लौटें

## S11 Get help with this task (a short problem code)

help-eyebrow = मदद और निजता
help-title = इस काम में मदद पाएं
help-body = एक छोटा कोड शेयर करें, जिससे किसी को पता चले कि क्या गड़बड़ हुई। पहले देख लें कि उसमें क्या शामिल है।
help-making = कोड बन रहा है…
help-code-title = समस्या कोड
# $code is the code itself, for example "PCT-7F3K-21". Keep it unchanged.
help-code =
    .aria-label = समस्या कोड { $code }
help-contains-title = इसमें क्या शामिल है
help-contains-problem = समस्या किस तरह की है और काम कहाँ तक पहुँचा।
help-contains-step = स्थानांतरण का वह चरण, जहाँ यह हुआ।
help-contains-versions = PCTwin का संस्करण, और यह लैपटॉप कौन-सा सिस्टम चलाता है, जैसे Windows या macOS।
help-never-title = इसमें कभी क्या शामिल नहीं होता
help-chip-included = शामिल
help-chip-excluded = कभी शामिल नहीं
help-never-names = आपका या किसी और का नाम।
help-never-files = आपकी फ़ाइलों में क्या है, या कोई भी पासवर्ड।
help-never-network = फ़ाइल या फ़ोल्डर के नाम, आपकी फ़ाइलें कहाँ रखी हैं, या Wi-Fi के नाम।
help-note = यह कोड किसी को आपके लैपटॉप तक पहुँच नहीं देता।
help-copy = कोड कॉपी करें
help-copied = कोड कॉपी हो गया।
help-share = शेयर करें
help-share-hint = इसे WhatsApp या किसी भी ऐप से भेजें।
help-share-text = मेरा PCTwin समस्या कोड { $code } है।
help-failed = समस्या कोड नहीं बन सका। फिर से कोशिश करें। अगर फिर भी काम न करे, तो समस्या अपने शब्दों में बताएं।
help-try-again = फिर से कोशिश करें
help-copy-failed = कोड कॉपी नहीं हो सका। ऊपर उसे चुनें और ख़ुद कॉपी करें।
help-home-eyebrow = मदद
help-home-title = आम समस्याएँ और क्या करें
help-home-body = जो चीज़ें अक्सर रुकावट बनती हैं, उनके लिए छोटी गाइड। ये बिना इंटरनेट के चलती हैं।
help-home-more = getpctwin.com पर और मदद
help-guide-find-title = लैपटॉप एक-दूसरे को नहीं ढूँढ पा रहे
help-guide-find-body = दोनों लैपटॉप एक ही Wi-Fi पर रखें, गेस्ट नेटवर्क पर नहीं। जोड़ते समय कोई VPN बंद करें। फिर भी न चले तो फ़ोन का हॉटस्पॉट चालू करके दोनों लैपटॉप उससे जोड़ें।
help-guide-mac-network-title = आपका Mac लोकल नेटवर्क के बारे में पूछता है
help-guide-mac-network-body = अनुमति दें चुनें, ताकि PCTwin आपका दूसरा लैपटॉप ढूँढ सके। अगर आपने अनुमति न दें चुना था, तो सिस्टम सेटिंग्स, फिर गोपनीयता और सुरक्षा, फिर लोकल नेटवर्क खोलें और PCTwin चालू करें।
help-guide-firewall-title = Windows फ़ायरवॉल के बारे में पूछता है
help-guide-firewall-body = निजी नेटवर्क, जैसे घर के Wi-Fi, पर PCTwin को अनुमति दें। PCTwin आपके अपने नेटवर्क पर सिर्फ़ आपके दूसरे लैपटॉप से बात करता है।
help-guide-stopped-title = स्थानांतरण बीच में रुक गया
help-guide-stopped-body = जो कॉपी और जाँचा जा चुका है, वह नहीं खोता, और पुराना लैपटॉप नहीं बदला। लैपटॉप फिर से जोड़ें: नए लैपटॉप पर पहले से मौजूद फ़ाइलें पहचानी जाती हैं और दोबारा कॉपी नहीं होतीं।
help-guide-others-title = दूसरे लोगों की चीज़ें ले जाना
help-guide-others-body = पुराने लैपटॉप पर «कौन जा रहा है?» में उन्हें चुनें। हर लैपटॉप पर एडमिनिस्ट्रेटर एक बार मंज़ूरी देता है। आप उनकी फ़ाइलें कभी नहीं देखते। नए खाते का अस्थायी पासवर्ड लैपटॉप की अपनी विंडो में एक बार दिखता है; वे पहली बार साइन इन पर अपना पासवर्ड चुनते हैं।
help-guide-code-title = किसी से मदद माँगना
help-guide-code-body = किसी भी समस्या वाले पेज पर «मदद लें» चुनें। PCTwin एक छोटा कोड बनाता है जो बताता है कि क्या गड़बड़ हुई, बिना किसी नाम, फ़ाइल या पासवर्ड के, जिसे आप WhatsApp या किसी भी ऐप से भेज सकते हैं।

## Label words (the name each laptop shows while pairing)

label-colour-red = लाल
label-colour-orange = नारंगी
label-colour-yellow =
    { $gender ->
        [feminine] पीली
       *[masculine] पीला
    }
label-colour-green =
    { $gender ->
        [feminine] हरी
       *[masculine] हरा
    }
label-colour-blue =
    { $gender ->
        [feminine] नीली
       *[masculine] नीला
    }
label-colour-purple = बैंगनी
label-colour-pink = गुलाबी
label-colour-brown =
    { $gender ->
        [feminine] भूरी
       *[masculine] भूरा
    }
label-colour-black =
    { $gender ->
        [feminine] काली
       *[masculine] काला
    }
label-colour-white = सफ़ेद
label-colour-grey = स्लेटी
label-colour-gold =
    { $gender ->
        [feminine] सुनहरी
       *[masculine] सुनहरा
    }

label-animal-ant = चींटी
label-animal-badger = बिज्जू
label-animal-bat = चमगादड़
label-animal-bear = भालू
label-animal-bee = मधुमक्खी
label-animal-beetle = गुबरैला
label-animal-bison = बाइसन
label-animal-butterfly = तितली
label-animal-camel = ऊँट
label-animal-cat = बिल्ली
label-animal-cheetah = चीता
label-animal-chicken = मुर्गी
label-animal-crab = केकड़ा
label-animal-crane = सारस
label-animal-crocodile = मगरमच्छ
label-animal-crow = कौआ
label-animal-deer = हिरन
label-animal-dolphin = डॉल्फ़िन
label-animal-donkey = गधा
label-animal-dove = फ़ाख़्ता
label-animal-dragonfly = ड्रैगनफ़्लाई
label-animal-duck = बत्तख़
label-animal-eagle = गरुड़
label-animal-elephant = हाथी
label-animal-falcon = शाहीन
label-animal-flamingo = फ़्लेमिंगो
label-animal-fox = लोमड़ी
label-animal-frog = मेंढक
label-animal-gazelle = चिकारा
label-animal-swordfish = तलवार मछली
label-animal-giraffe = जिराफ़
label-animal-goat = बकरी
label-animal-goose = कलहंस
label-animal-gorilla = गोरिल्ला
label-animal-hamster = हैम्स्टर
label-animal-hawk = बाज़
label-animal-hedgehog = हेजहॉग
label-animal-armadillo = आर्मडिलो
label-animal-hippo = दरियाई घोड़ा
label-animal-horse = घोड़ा
label-animal-hummingbird = हमिंगबर्ड
label-animal-jaguar = जैगुआर
label-animal-jellyfish = जेलीफ़िश
label-animal-kangaroo = कंगारू
label-animal-kingfisher = किलकिला
label-animal-koala = कोआला
label-animal-ladybird = लेडीबग
label-animal-lemur = लीमर
label-animal-leopard = तेंदुआ
label-animal-lion = शेर
label-animal-lizard = छिपकली
label-animal-llama = लामा
label-animal-lobster = झींगा
label-animal-lynx = वनबिलाव
label-animal-macaw = मकाउ
label-animal-meerkat = मीरकैट
label-animal-mole = छछूंदर
label-animal-moose = मूस
label-animal-pufferfish = पफ़रफ़िश
label-animal-mouse = चूहा
label-animal-octopus = ऑक्टोपस
label-animal-ostrich = शुतुरमुर्ग
label-animal-otter = ऊदबिलाव
label-animal-owl = उल्लू
label-animal-panda = पांडा
label-animal-kiwi = कीवी
label-animal-parrot = तोता
label-animal-peacock = मोर
label-animal-pelican = पेलिकन
label-animal-penguin = पेंगुइन
label-animal-puffin = पफ़िन
label-animal-rabbit = ख़रगोश
label-animal-raccoon = रैकून
label-animal-rhino = गैंडा
label-animal-robin = रॉबिन
label-animal-salmon = सैल्मन
label-animal-seahorse = समुद्री घोड़ा
label-animal-seal = सील
label-animal-shark = शार्क
label-animal-sheep = भेड़
label-animal-sloth = आलसी भालू
label-animal-snail = घोंघा
label-animal-sparrow = गौरैया
label-animal-squirrel = गिलहरी
label-animal-starfish = तारामछली
label-animal-stork = लकलक
label-animal-swan = हंस
label-animal-tiger = बाघ
label-animal-snake = साँप
label-animal-toucan = टूकन
label-animal-turtle = कछुआ
label-animal-walrus = वालरस
label-animal-whale = व्हेल
label-animal-wolf = भेड़िया
label-animal-woodpecker = कठफोड़वा
label-animal-yak = याक
label-animal-zebra = ज़ेबरा
label-animal-dog = कुत्ता
label-animal-cow = गाय
label-animal-beaver = बीवर

# Grammatical gender of each animal noun above (value is a fixed keyword, never translated).
label-gender-ant = feminine
label-gender-badger = masculine
label-gender-bat = masculine
label-gender-bear = masculine
label-gender-bee = feminine
label-gender-beetle = masculine
label-gender-bison = masculine
label-gender-butterfly = feminine
label-gender-camel = masculine
label-gender-cat = feminine
label-gender-cheetah = masculine
label-gender-chicken = feminine
label-gender-crab = masculine
label-gender-crane = masculine
label-gender-crocodile = masculine
label-gender-crow = masculine
label-gender-deer = masculine
label-gender-dolphin = feminine
label-gender-donkey = masculine
label-gender-dove = feminine
label-gender-dragonfly = feminine
label-gender-duck = feminine
label-gender-eagle = masculine
label-gender-elephant = masculine
label-gender-falcon = masculine
label-gender-flamingo = masculine
label-gender-fox = feminine
label-gender-frog = masculine
label-gender-gazelle = masculine
label-gender-swordfish = feminine
label-gender-giraffe = masculine
label-gender-goat = feminine
label-gender-goose = masculine
label-gender-gorilla = masculine
label-gender-hamster = masculine
label-gender-hawk = masculine
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
label-gender-ladybird = masculine
label-gender-lemur = masculine
label-gender-leopard = masculine
label-gender-lion = masculine
label-gender-lizard = feminine
label-gender-llama = masculine
label-gender-lobster = masculine
label-gender-lynx = masculine
label-gender-macaw = masculine
label-gender-meerkat = masculine
label-gender-mole = feminine
label-gender-moose = masculine
label-gender-pufferfish = feminine
label-gender-mouse = masculine
label-gender-octopus = masculine
label-gender-ostrich = masculine
label-gender-otter = masculine
label-gender-owl = masculine
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
label-gender-salmon = feminine
label-gender-seahorse = masculine
label-gender-seal = feminine
label-gender-shark = feminine
label-gender-sheep = feminine
label-gender-sloth = masculine
label-gender-snail = masculine
label-gender-sparrow = feminine
label-gender-squirrel = feminine
label-gender-starfish = feminine
label-gender-stork = masculine
label-gender-swan = masculine
label-gender-tiger = masculine
label-gender-snake = masculine
label-gender-toucan = masculine
label-gender-turtle = masculine
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
# Hindi: colour first, agreeing in gender with the animal.
label-picked = { $colour } { $animal }
label-unnamed = बिना नाम का लैपटॉप

## M02 Finding your files

scan-eyebrow = आपका स्थानांतरण
scan-title = आपकी फ़ाइलें खोजी जा रही हैं
scan-body = PCTwin जो भी ढूंढता है, वह सहेजा जाता है, ताकि खोज रुकने पर भी आगे जारी रह सके।
scan-progress =
    .aria-label = आपकी चुनी जगहों में खोज हो रही है
# $size is passed but not shown in the mockup line.
scan-found =
    { NUMBER($count) ->
        [one] अब तक { $count } फ़ाइल मिली।
       *[other] अब तक { $count } फ़ाइलें मिलीं।
    }
scan-step-usual-title = फ़ाइलें और फ़ोल्डर
scan-step-usual-body = वे जगहें खोजी गईं जिन्हें PCTwin पढ़ सकता है।
scan-step-other-title = दूसरे फ़ोल्डर और ड्राइव
scan-step-other-body = सामान्य जगहों के बाहर इस्तेमाल होने वाले फ़ोल्डर और ड्राइव।
scan-step-protected-title = संरक्षित जगहें
scan-step-protected-body =
    { NUMBER($count) ->
        [one] { $count } जगह के लिए अब भी आपकी अनुमति चाहिए।
       *[other] { $count } जगहों के लिए अब भी आपकी अनुमति चाहिए।
    }
scan-step-done = पूरा
scan-step-current = अभी
scan-step-todo = आगे
scan-unreadable-note = जिन जगहों को PCTwin पढ़ नहीं सका, वे सूची में रहती हैं। उन्हें खोजी गई जगहों में नहीं गिना जाता।
scan-status-not-searched = खोजा नहीं गया
scan-paused = खोज रोकी गई। अब तक मिली हर चीज़ सहेजी गई है।
scan-resumed = खोज वहीं से फिर शुरू हुई जहाँ रुकी थी।
scan-partial =
    { NUMBER($count) ->
        [one] { $count } संरक्षित जगह अभी खोजी नहीं गई है।
       *[other] { $count } संरक्षित जगहें अभी खोजी नहीं गई हैं।
    }
scan-allow = पहुँच की अनुमति दें
scan-pause = रोकें
scan-resume = खोज फिर से शुरू करें
scan-continue = देखें क्या मिला

## M03 Everything you chose, in view

overview-eyebrow = आपका स्थानांतरण
overview-title = PCTwin को क्या मिला
overview-body = योजना बनने से पहले देखें कि PCTwin को क्या मिला।
# The metric value next to each label is passed in already formatted.
overview-size-label = चुनी गई फ़ाइलें
overview-items-label =
    { NUMBER($count) ->
        [one] फ़ाइल
       *[other] फ़ाइलें
    }
overview-attention-label =
    { NUMBER($count) ->
        [one] पर ध्यान देना ज़रूरी
       *[other] पर ध्यान देना ज़रूरी
    }
overview-files-title = फ़ाइलें और फ़ोल्डर
overview-files-body = आपके चुने दस्तावेज़, फ़ोटो और दूसरे फ़ोल्डर।
overview-dest-title = कहाँ जाएंगे
overview-dest-body = नए लैपटॉप पर हर फ़ोल्डर की जगह।
overview-people-title = लोग
overview-people-body = हर व्यक्ति के लिए नए लैपटॉप पर अकाउंट चुनें।
overview-review = देखें
overview-partial =
    { NUMBER($count) ->
        [one] { $count } संरक्षित जगह अभी खोजी नहीं गई है।
       *[other] { $count } संरक्षित जगहें अभी खोजी नहीं गई हैं।
    }
overview-allow = पहुँच की अनुमति दें
overview-continue = आगे बढ़ें

## M04 Keep what matters to you

files-eyebrow = आपका स्थानांतरण
files-title = ले जाने के लिए फ़ोल्डर चुनें
files-body = फ़ोल्डर, दूसरी ड्राइव और कोई भी दूसरी जगह चुनें।
files-search-label = फ़ोल्डर ढूंढें
files-search-placeholder = फ़ोल्डर खोजें
files-select =
    .aria-label = { $name } ले जाएं
files-folder-detail =
    { NUMBER($count) ->
        [one] { $size } · { $count } फ़ाइल
       *[other] { $size } · { $count } फ़ाइलें
    }
files-status-protected = अनुमति चाहिए
files-protected-body = इस फ़ोल्डर को पढ़ने के लिए PCTwin को आपकी अनुमति चाहिए। अनुमति मिलने तक इसकी फ़ाइलें गिनी नहीं जातीं।
files-allow = पहुँच की अनुमति दें
files-status-added = आपने शामिल किया
files-add = एक और फ़ोल्डर शामिल करें
files-remove =
    .aria-label = { $name } हटाएं
files-no-limit = मुफ़्त स्थानांतरण में फ़ाइल के साइज़ या फ़ोल्डरों की गिनती की कोई सीमा नहीं है।
files-total =
    { NUMBER($count) ->
        [one] { $count } फ़ोल्डर चुना गया, कुल { $size }
       *[other] { $count } फ़ोल्डर चुने गए, कुल { $size }
    }
files-none = आगे बढ़ने के लिए कम से कम एक फ़ोल्डर चुनें।
files-continue = जगहें चुनें

## M05 Give every file a home

dest-eyebrow = आपका स्थानांतरण
dest-title = आपके फ़ोल्डर कहाँ जाएँगे
dest-body = चुनें कि नए लैपटॉप पर हर फ़ोल्डर कहाँ जाए।
dest-col-chosen = आपने क्या चुना
dest-col-place = कहाँ जाएगा
dest-col-placement = ड्राइव
dest-placement-local = अंदरूनी
dest-placement-external = बाहरी
dest-col-status = स्थिति
dest-select =
    .aria-label = { $name } कहाँ जाए
# $folder is a folder name on the new laptop, for example "Documents".
dest-option-folder = नया लैपटॉप / { $folder }
# $drive is the name of a drive connected to the new laptop.
dest-option-drive = बाहरी ड्राइव / { $drive }
dest-option-choose = फ़ोल्डर चुनें
dest-default-label = सामान्य जगह
dest-status-ready = तैयार
dest-status-missing = ड्राइव नहीं मिली
dest-status-space = पर्याप्त स्टोरेज नहीं
dest-all-ready = नए लैपटॉप पर हर फ़ोल्डर की जगह तय है।
dest-missing = { $drive } नए लैपटॉप से कनेक्ट नहीं है। इसे कनेक्ट करें, या इन फ़ाइलों के लिए कोई दूसरी जगह चुनें।
dest-space = { $place } पर { $free } स्टोरेज उपलब्ध है, लेकिन इन फ़ाइलों को { $needed } चाहिए। वहाँ स्टोरेज खाली करें, कोई दूसरी जगह चुनें, या कम फ़ाइलें भेजें।
dest-change = बदलें
dest-continue = योजना देखें

## M23 Approve the move you want

plan-eyebrow = आपका स्थानांतरण
plan-title = योजना जाँचें और मंज़ूर करें
plan-body = PCTwin सिर्फ़ वही करता है जो इस योजना में लिखा है।
plan-what-title = आपने क्या चुना
# $size is already formatted, for example "120 GB". $count is passed but not shown.
plan-what-body = आपके चुने फ़ोल्डरों से { $size } फ़ाइलें, आपकी चुनी जगहों पर।
plan-where-body =
    { NUMBER($count) ->
        [one] नए लैपटॉप पर { $count } जगह पर।
       *[other] नए लैपटॉप पर { $count } जगहों पर।
    }
plan-hand-title = शुरू करने से पहले
plan-hand-body =
    { NUMBER($count) ->
        [one] { $count } चरण। यह आख़िर में आपकी रिपोर्ट में दिखेगा।
       *[other] { $count } चरण। ये आख़िर में आपकी रिपोर्ट में दिखेंगे।
    }
plan-hand-none = कुछ नहीं, जहाँ तक PCTwin अभी बता सकता है।
plan-old-safe = आपके पुराने लैपटॉप में कुछ नहीं बदलता। फ़ाइलें कॉपी की जाती हैं, उससे कभी हटाई नहीं जातीं।
plan-edit = योजना बदलें
plan-edit-note = योजना बदलने पर, कुछ भी नया शुरू होने से पहले उसे फिर से मंज़ूर करना होगा।
plan-approve = यह योजना मुझे मंज़ूर है, उन कामों समेत जो मुझे ख़ुद करने हैं।
plan-not-approved = यह योजना अभी मंज़ूर नहीं हुई है, इसलिए कुछ भी नहीं जाएगा।
plan-changed = योजना बदल गई है, इसलिए कुछ भी नया जाने से पहले इसे फिर से मंज़ूर करें।
plan-start = स्थानांतरण शुरू करें

## Plan screens: shared words and the lines the copy deck did not cover

plan-back = वापस
# $n is a count of files or folders.
plan-number = { NUMBER($n) }
# $n is a number already rounded for reading, for example 18.4.
plan-size-b = { NUMBER($n) } बाइट
plan-size-kb = { NUMBER($n) } KB
plan-size-mb = { NUMBER($n) } MB
plan-size-gb = { NUMBER($n) } GB
plan-size-tb = { NUMBER($n) } TB
# $size is a size already written out, for example "18.4 GB". Used when part of a folder could not be read.
plan-size-at-least = कम से कम { $size }
plan-size-unknown = साइज़ पता नहीं
plan-role-home = होम फ़ोल्डर
plan-role-desktop = डेस्कटॉप
plan-role-documents = दस्तावेज़
plan-role-downloads = डाउनलोड
plan-role-pictures = चित्र
plan-role-music = संगीत
plan-role-videos = वीडियो
plan-role-public = सार्वजनिक फ़ोल्डर
plan-role-app-data = ऐप डेटा
plan-role-other = दूसरा फ़ोल्डर
# $done and $total are counts of places this search has looked at and has to look at.
scan-places = { $total } में से { $done } जगहें खोजी गईं
scan-card-title = आपकी चुनी जगहों में खोज हो रही है
scan-pausing = अभी वाला फ़ोल्डर पूरा होने के बाद खोज रुकेगी। अब तक मिली हर चीज़ सहेजी गई है।
overview-row-attention = ध्यान देना ज़रूरी
files-status-work = दफ़्तर या स्कूल
files-work-body = यह फ़ोल्डर आपके दफ़्तर या स्कूल के नियंत्रण में है, इसलिए टिक किए बिना इसे शामिल नहीं किया जाता।
# $count is how many files in the folder are only online.
files-online-only =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल सिर्फ़ ऑनलाइन है, इसलिए वह कॉपी नहीं होती।
       *[other] { $count } फ़ाइलें सिर्फ़ ऑनलाइन हैं, इसलिए वे कॉपी नहीं होतीं।
    }
files-no-match = इस नाम का कोई फ़ोल्डर नहीं है।
dest-status-none = जगह चुनें
dest-status-readonly = इसमें लिखा नहीं जा सकता
dest-status-format = फ़ाइल बहुत बड़ी
# $name is the folder that needs a place.
dest-none = नए लैपटॉप पर { $name } के लिए जगह चुनें।
# $place is a drive on the new laptop that cannot be written to.
dest-readonly = { $place } सिर्फ़ पढ़ने के लिए है, इसलिए उस पर कुछ भी कॉपी नहीं हो सकता। कोई दूसरी जगह चुनें।
# $place is the drive. Some drive formats cannot hold a single very large file.
dest-format = इन फ़ोल्डरों की एक फ़ाइल इतनी बड़ी है कि { $place } जिस तरह सेट है, उसमें नहीं आ सकती। कोई दूसरी जगह चुनें।
# $folder is a folder on the new laptop and $drive the drive it is on.
plan-place = { $drive } पर { $folder }
plan-group-to = { $place } पर जाएगा
plan-group-move = { $folder } ले जाएँ
plan-place-change = कोई दूसरी जगह चुनें
plan-place-usual = सामान्य जगह इस्तेमाल करें
plan-place-free = वहाँ { $size } खाली है
plan-place-no-room = वहाँ पर्याप्त खाली जगह नहीं है। कोई दूसरी जगह चुनें, या इस फ़ोल्डर से निशान हटा दें।
plan-place-no-safe-name = PCTwin उस ड्राइव पर फ़ाइलें सुरक्षित रूप से नहीं रख सकता। कोई दूसरी जगह चुनें।
plan-place-cannot-open = PCTwin वह फ़ोल्डर नहीं खोल सका। कोई दूसरी जगह चुनें।
plan-place-locked = वह ड्राइव लॉक है। पहले उसे अनलॉक करें, फिर उसे दोबारा चुनें।
plan-version = योजना वर्शन { $revision }
plan-details = और जानकारी
plan-preview-title = शुरू करने से पहले
plan-preview-fits = इस लैपटॉप में पर्याप्त जगह खाली है।
plan-preview-fits-about = इस लैपटॉप में पर्याप्त खाली जगह होनी चाहिए। इसमें PCTwin द्वारा इंस्टॉल किए जाने वाले ऐप्स और उनके डेटा के लिए लगभग { $extra } गिना गया है।
# Added 23 October 2026: what moves first, from the move's own order. $places is a list of folder names.
plan-preview-first = ये पहले ले जाए जाएँगे, ताकि आप अपना नया लैपटॉप जल्दी इस्तेमाल कर सकें: { $places }।
plan-preview-short = इस लैपटॉप को { $size } और खाली जगह चाहिए। शुरू करने से पहले कुछ जगह खाली करें, वरना ड्राइव भरने पर स्थानांतरण रुक जाएगा।
plan-preview-short-about = इस लैपटॉप को लगभग { $size } और खाली जगह चाहिए, जिसमें PCTwin द्वारा इंस्टॉल किए जाने वाले ऐप्स और उनके डेटा के लिए लगभग { $extra } गिना गया है। शुरू करने से पहले कुछ जगह खाली करें, वरना ड्राइव भरने पर स्थानांतरण रुक जाएगा।
plan-preview-space-unknown = PCTwin यह नहीं पढ़ सका कि इस लैपटॉप में कितनी जगह खाली है।
plan-preview-synced = इस लैपटॉप पर { $folder } को { $service } में रखा जाता है। वहाँ ले जाई गई फ़ाइलें { $service } पर अपलोड होंगी और वहाँ आपकी जगह इस्तेमाल करेंगी।
plan-preview-time-one =
    { NUMBER($minutes) ->
        [one] लगभग { $minutes } मिनट।
       *[other] लगभग { $minutes } मिनट।
    }
plan-preview-time = लगभग { $low } से { $high } मिनट।
plan-preview-renamed =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल का नाम थोड़ा बदलेगा, क्योंकि यह लैपटॉप उसका नाम स्वीकार नहीं करता। रिपोर्ट में यह लिखा होगा।
       *[other] { $count } फ़ाइलों के नाम थोड़े बदलेंगे, क्योंकि यह लैपटॉप उनके नाम स्वीकार नहीं करता। रिपोर्ट में हर एक लिखी होगी।
    }
plan-preview-too-big =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल इस लैपटॉप की ड्राइव के लिए बहुत बड़ी है और कॉपी नहीं होगी।
       *[other] { $count } फ़ाइलें इस लैपटॉप की ड्राइव के लिए बहुत बड़ी हैं और कॉपी नहीं होंगी।
    }
plan-preview-refused =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल का नाम या फ़ोल्डर यह लैपटॉप स्वीकार नहीं करता, इसलिए वह कॉपी नहीं होगी।
       *[other] { $count } फ़ाइलों के नाम या फ़ोल्डर यह लैपटॉप स्वीकार नहीं करता, इसलिए वे कॉपी नहीं होंगी।
    }
plan-preview-runs-at-sign-in =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल इस लैपटॉप पर किसी के साइन इन करते ही अपने-आप चल पड़ेगी। देख लें कि आप यही चाहते हैं।
       *[other] { $count } फ़ाइलें इस लैपटॉप पर किसी के साइन इन करते ही अपने-आप चल पड़ेंगी। देख लें कि आप यही चाहते हैं।
    }
# $reference is a short code that names exactly this version of the plan.
plan-reference = संदर्भ { $reference }
# $count is how many files are affected in each line below.
plan-hand-cloud-only =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल सिर्फ़ ऑनलाइन है। PCTwin उसे डाउनलोड नहीं करता। पहले उसे पुराने लैपटॉप पर उपलब्ध कराएं, या उसे छोड़ दें।
       *[other] { $count } फ़ाइलें सिर्फ़ ऑनलाइन हैं। PCTwin उन्हें डाउनलोड नहीं करता। पहले उन्हें पुराने लैपटॉप पर उपलब्ध कराएं, या उन्हें छोड़ दें।
    }
plan-hand-unreadable =
    { NUMBER($count) ->
        [one] { $count } आइटम खुल नहीं सका, इसलिए वह नहीं जा रहा। पहुँच की अनुमति दें, या उसे छोड़ दें।
       *[other] { $count } आइटम खुल नहीं सके, इसलिए वे नहीं जा रहे। पहुँच की अनुमति दें, या उन्हें छोड़ दें।
    }
plan-hand-work-managed =
    { NUMBER($count) ->
        [one] { $count } आइटम आपके दफ़्तर या स्कूल के नियंत्रण में है, इसलिए छोड़ा गया है। इसे शामिल करने के लिए इसके फ़ोल्डर पर टिक करें।
       *[other] { $count } आइटम आपके दफ़्तर या स्कूल के नियंत्रण में हैं, इसलिए छोड़े गए हैं। इन्हें शामिल करने के लिए इनके फ़ोल्डरों पर टिक करें।
    }
plan-hand-too-many-files =
    { NUMBER($count) ->
        [one] { $count } फ़ोल्डर में इतनी फ़ाइलें हैं जितनी एक बार में नहीं ले जाई जा सकतीं, इसलिए यह नहीं जा रहा है। इस बार के बाद इसे क्विक सेंड से अलग भेजें, या USB ड्राइव से कॉपी करें।
       *[other] { $count } फ़ोल्डरों में इतनी फ़ाइलें हैं जितनी एक बार में नहीं ले जाई जा सकतीं, इसलिए ये नहीं जा रहे हैं। इस बार के बाद हर एक को क्विक सेंड से अलग भेजें, या USB ड्राइव से कॉपी करें।
    }
plan-blocked-someone-unmapped = पुराने लैपटॉप के किसी व्यक्ति के लिए नए लैपटॉप पर अभी कोई अकाउंट नहीं चुना गया है। चुनें कि हर व्यक्ति की फ़ाइलें कहाँ जाएं, फिर वापस आएं।
plan-blocked-account-used-twice = नए लैपटॉप पर दो लोग एक ही अकाउंट में पहुँच जाएंगे। हर व्यक्ति को उसका अपना अकाउंट दें।
plan-blocked-invalid = यह योजना जैसी है, वैसी मंज़ूर नहीं की जा सकती। वापस जाकर अपने विकल्प जाँचें।
plan-blocked-fix = अकाउंट चुनें

## Tray menu

tray-show = PCTwin दिखाएं
tray-send = फ़ाइलें भेजें
tray-quit = PCTwin बंद करें

## F01 secondary actions and their coming-soon pages

welcome-restore = सेटअप वापस लाएं
welcome-rescue = लैपटॉप बचाएं
welcome-quick-send = क्विक सेंड

soon-eyebrow = जल्द आ रहा है
soon-restore-title = सेटअप वापस लाएं
soon-restore-body = स्थानांतरण के बाद सहेजी गई सेटअप प्रोफ़ाइल से किसी लैपटॉप को फिर से तैयार करें। यह बाद के किसी वर्शन में आएगा।
soon-rescue-title = लैपटॉप बचाएं
soon-rescue-body = जो लैपटॉप चालू नहीं होता, उसकी ड्राइव किसी चालू लैपटॉप से कनेक्ट करके फ़ाइलें वापस लाएं। यह बाद के किसी वर्शन में आएगा।
soon-move-title = पूरा स्थानांतरण
soon-move-body = अपने पुराने लैपटॉप से सब कुछ नए लैपटॉप पर ले जाएं: आपकी फ़ाइलें, ऐप्स और सेटिंग्स। यह बाद के किसी वर्शन में आएगा। अभी के लिए, फ़ाइलें भेजने के लिए क्विक सेंड का उपयोग करें।

## X01 Move progress, compact

progress-eyebrow = आपका स्थानांतरण
progress-title = आपकी चीज़ें इस लैपटॉप पर ले जाई जा रही हैं
progress-title-old = आपकी चीज़ें आपके नए लैपटॉप पर भेजी जा रही हैं
progress-title-paused = स्थानांतरण रुका हुआ है
progress-paired = जोड़ा गया
progress-percent =
    .aria-label = स्थानांतरण की प्रगति
# $done and $total are already formatted sizes, for example "81.6 GB" and "120 GB".
progress-amount = { $total } में से { $done }
# $range is a measured range, already formatted, for example "15 to 20 minutes".
progress-time-left = लगभग { $range } बाकी
progress-time-measuring = बाकी समय का अंदाज़ा लगाया जा रहा है…
# $speed is measured and already formatted, for example "62 MB/s".
progress-speed = { $speed }, मापा गया
progress-count-needs-action =
    { NUMBER($count) ->
        [one] { $count } आइटम पर आपको कुछ करना है
       *[other] { $count } आइटम पर आपको कुछ करना है
    }
progress-current-copying = अभी कॉपी हो रहा है: { $name }
progress-current-checking = अभी जाँच हो रही है: { $name }
progress-needs-action =
    { NUMBER($count) ->
        [one] { $count } आइटम पर आपको कुछ करना है। बाकी का स्थानांतरण चलता रहता है।
       *[other] { $count } आइटम पर आपको कुछ करना है। बाकी का स्थानांतरण चलता रहता है।
    }
progress-see-needs-action = देखें आपको क्या करना है
progress-interrupted = दूसरे लैपटॉप से कनेक्शन टूट गया। PCTwin फिर से कनेक्ट करने की कोशिश कर रहा है। जो कॉपी और जाँचा जा चुका है, उसमें से कुछ नहीं खोया।
progress-low-battery = इस लैपटॉप की बैटरी कम है। इसे चार्ज पर लगाएँ ताकि स्थानांतरण न रुके।
progress-battery-paused = इस लैपटॉप की बैटरी लगभग ख़त्म है, इसलिए स्थानांतरण सुरक्षित रूप से रुका है। इसे चार्ज पर लगाएँ, फिर जारी रखें। कुछ भी नहीं खोया।
progress-old-safe = आपके पुराने लैपटॉप में कुछ नहीं बदलता।
progress-pause = स्थानांतरण रोकें
progress-compact = कम दिखाएँ
progress-expand = हर आइटम दिखाएँ
progress-stop = स्थानांतरण बंद करें

## X02 Move progress, expanded

progress-all-title = स्थानांतरण का हर आइटम
progress-all-body = हर आइटम दिखाता है कि वह कहाँ से आ रहा है और उसकी स्थिति क्या है।
progress-metric-overall = कुल प्रगति
progress-metric-committed = कॉपी और जाँचा गया
progress-metric-estimate = अनुमान
progress-col-item = आइटम
progress-col-from = कहाँ से
progress-col-status = स्थिति
progress-state-waiting = इंतज़ार
progress-state-copying = कॉपी हो रहा है
progress-state-verified = जाँचा गया
progress-state-checking = जाँच हो रही है
progress-state-paused = रुका हुआ
progress-checked-note = कोई आइटम जाँच होने के बाद ही कॉपी हुआ माना जाता है।
# Added 23 October 2026: T14, one tip under the progress bar, about every 12 seconds.
tips-label = सुझाव
tips-pause = सुझाव रोकें
tips-resume = और सुझाव दिखाएँ
tips-next = अगला सुझाव
tip-battery = यह लैपटॉप बैटरी पर चल रहा है। हो सके तो इसे प्लग इन करें, ताकि लंबा स्थानांतरण बीच में न रुके।
tip-closer = कनेक्शन धीमा है। लैपटॉप को Wi-Fi राउटर के पास ले जाने से यह तेज़ हो सकता है।
tip-lids = इस लैपटॉप का ढक्कन खुला रखें। ढक्कन बंद करने से यह स्लीप में जा सकता है, जिससे स्थानांतरण रुक जाता है।
tip-keep-using = आप इस लैपटॉप का इस्तेमाल करते रह सकते हैं। PCTwin इससे सिर्फ़ पढ़ता है।
tip-pause = आप कभी भी स्थानांतरण रोक सकते हैं और बाद में जारी रख सकते हैं।
tip-power-cut = अगर बिजली चली जाए, तो दोनों लैपटॉप चालू करें और PCTwin खोलें। यह वहीं से जारी रखता है जहाँ रुका था, और जो कॉपी और जाँचा जा चुका है, उसमें से कुछ नहीं खोता।

## F09 Mini panel

mini-title = स्थानांतरण जारी है
mini-title-paused = स्थानांतरण रुका हुआ है
mini-what = आपकी चुनी फ़ाइलें भेजी जा रही हैं
mini-progress =
    .aria-label = स्थानांतरण की प्रगति
mini-pause = रोकें
mini-resume = फिर से शुरू करें
mini-open = स्थानांतरण खोलें
# Used when lane E reuses this panel for a Quick Send.
mini-title-send = फ़ाइलें भेजी जा रही हैं
mini-what-send = चुनी गई फ़ाइलें भेजी जा रही हैं
mini-open-send = भेजना खोलें
mini-progress-send =
    .aria-label = भेजने की प्रगति

## X05 Paused at a safe point

pause-title = स्थानांतरण रोका गया है
pause-title-finishing = स्थानांतरण रोका जा रहा है…
pause-body = कॉपी और जाँची गई हर चीज़ रखी गई है। बाकी इंतज़ार कर रहा है।
pause-finishing = पहले { $name } पूरा किया जा रहा है, ताकि वह आधा कॉपी होकर न रह जाए।
pause-kept-title = फ़ाइलें
pause-kept-body = अब तक कॉपी और जाँची गई हर चीज़ रखी गई है।
pause-waiting-title = इंतज़ार
pause-waiting-body =
    { NUMBER($count) ->
        [one] { $count } आइटम अभी कॉपी होना बाकी है। कुछ भी नहीं खोया।
       *[other] { $count } आइटम अभी कॉपी होने बाकी हैं। कुछ भी नहीं खोया।
    }
pause-status-kept = रुका हुआ
pause-status-waiting = इंतज़ार
pause-note = रोकने से कुछ भी वापस नहीं होता। आपके पुराने लैपटॉप में कुछ नहीं बदलता।
pause-resume = स्थानांतरण फिर से शुरू करें

## X10 Stop the remaining move? (danger dialog)

cancel-title = बाकी स्थानांतरण बंद करें?
cancel-body = बंद करने पर कॉपी और जाँची गई हर चीज़ रहती है। इससे स्थानांतरण वापस नहीं होता।
cancel-copied-title = कॉपी हुई फ़ाइलें
cancel-copied-body = नए लैपटॉप पर रहती हैं।
cancel-rest-title = बाकी स्थानांतरण
cancel-rest-body = कुछ भी नया शुरू नहीं होता। जो फ़ाइलें अभी कॉपी नहीं हुईं, वे सिर्फ़ पुराने लैपटॉप पर रहती हैं।
cancel-partial-title = आधी कॉपी हुई फ़ाइल
cancel-partial-body = उसे कभी पूरी फ़ाइल की तरह नहीं रखा जाता।
cancel-old-title = पुराना लैपटॉप
cancel-old-body = इसमें कुछ नहीं बदलता।
cancel-ack = मुझे पता है कि बंद करने से कॉपी हुई चीज़ें वापस नहीं होतीं
cancel-stop = बंद करें और रिपोर्ट देखें
cancel-keep = स्थानांतरण जारी रखें

## R01 Welcome to your next laptop (outcome)

outcome-eyebrow = स्थानांतरण के नतीजे
outcome-title = स्थानांतरण पूरा हो गया
outcome-body = आपने जो कुछ मंज़ूर किया था, उसके साथ क्या हुआ, यहाँ देखें।
outcome-title-old = स्थानांतरण पूरा हो गया
outcome-body-old = आपने जो कुछ भेजा, उसके साथ क्या हुआ, यहाँ देखें। यह लैपटॉप जैसा था वैसा ही है।
outcome-files-title = चुनी गई फ़ाइलें
outcome-status-partial = कुछ हिस्सा पूरा
# Added 23 October 2026: the after-move check on R01, counted the way the scan counts.
outcome-kinds-title = स्थानांतरण के बाद जाँचा गया
outcome-kind-photos = फ़ोटो
outcome-kind-videos = वीडियो
outcome-kind-music = संगीत
outcome-kind-documents = दस्तावेज़
outcome-kind-other = दूसरी फ़ाइलें
outcome-kind-count = { $planned } में से { $copied } कॉपी और जाँचे गए
outcome-next-title = आगे क्या करें
outcome-old-safe = जो पूरा नहीं हुआ, वह यहाँ सूची में रहता है। आपके पुराने लैपटॉप से कुछ भी नहीं हटाया गया।
outcome-landed-title = आपकी चीज़ें अब कहाँ हैं
outcome-report = पूरी रिपोर्ट देखें
outcome-home = होम पर लौटें
outcome-person-copied =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल कॉपी हुई।
       *[other] { $count } फ़ाइलें कॉपी हुईं।
    }
outcome-person-not-done =
    { NUMBER($count) ->
        [one] { $count } पूरी नहीं हुई।
       *[other] { $count } पूरी नहीं हुईं।
    }
outcome-person-at-sign-in = जब { $name } अगली बार इस लैपटॉप पर साइन इन करेंगे, तब ये अपनी जगह होंगी।
move-fallback-folder = पुराने लैपटॉप से
move-stop = स्थानांतरण रोकें
move-before-open = इंतज़ार के दौरान: यह लैपटॉप छोड़ने से पहले
move-get-help = मदद लें
pickup-eyebrow = आपका स्थानांतरण
pickup-title = रुके हुए स्थानांतरण को व्यवस्थित करें
pickup-body = इस लैपटॉप पर एक स्थानांतरण बीच में रुक गया, शायद लैपटॉप फिर से चालू हुआ या बिजली चली गई। पहले व्यवस्थित करें, फिर आगे बढ़ने के लिए लैपटॉप दोबारा जोड़ें।
pickup-unfinished-title = अधूरी कॉपियाँ
pickup-unfinished-body =
    { NUMBER($count) ->
        [one] स्थानांतरण रुकने के समय { $count } फ़ाइल कॉपी हो रही थी।
       *[other] स्थानांतरण रुकने के समय { $count } फ़ाइलें कॉपी हो रही थीं।
    }
pickup-unfinished-badge = व्यवस्थित करना है
pickup-tidy = व्यवस्थित करें
pickup-tidying = व्यवस्थित किया जा रहा है। पूरी कॉपियाँ जाँचकर अपनी जगह रखी जाती हैं; आपकी कोई चीज़ नहीं बदलती।
pickup-done = व्यवस्थित करना पूरा हुआ। जारी रखने के लिए लैपटॉप फिर से जोड़ें: जो पहले से यहाँ है वह मिल जाता है और दो बार कॉपी नहीं होता।
pickup-failed = इस बार व्यवस्थित करना काम नहीं किया। आपकी कोई चीज़ नहीं बदली। फिर से कोशिश करें, या पहले इस लैपटॉप पर PCTwin बंद करके दोबारा खोलें।
pickup-put-in-place = जगह पर रखी गईं
pickup-kept = आगे के लिए रखी गईं
pickup-not-finished =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल पूरी नहीं हो सकी। अगला स्थानांतरण इसे फिर से कॉपी करेगा।
       *[other] { $count } फ़ाइलें पूरी नहीं हो सकीं। अगला स्थानांतरण इन्हें फिर से कॉपी करेगा।
    }
pickup-left-for-later =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल ऐसी ड्राइव पर है जो अभी जुड़ी नहीं है। उसे जोड़कर फिर से व्यवस्थित करें।
       *[other] { $count } फ़ाइलें ऐसी ड्राइव पर हैं जो अभी जुड़ी नहीं है। उसे जोड़कर फिर से व्यवस्थित करें।
    }
pickup-later = अभी नहीं
ready-eyebrow = आपके पुराने लैपटॉप से
ready-title = आपकी चीज़ें आपके फ़ोल्डरों में हैं
ready-body = आपके पुराने लैपटॉप की चीज़ें इस लैपटॉप पर आ गई हैं, और अब आपके अपने फ़ोल्डरों में हैं।
ready-landed = फ़ाइलें आपके फ़ोल्डरों में
ready-where-title = ये कहाँ हैं
ready-where-body = दस्तावेज़, चित्र, संगीत, वीडियो और आपके दूसरे सामान्य फ़ोल्डरों में, जैसे पुराने लैपटॉप पर थे।
ready-kept =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल अलग रखी गई, क्योंकि उस नाम की फ़ाइल पहले से यहाँ थी। दोनों रखी गई हैं।
       *[other] { $count } फ़ाइलें अलग रखी गईं, क्योंकि उन नामों की फ़ाइलें पहले से यहाँ थीं। सब रखी गई हैं।
    }
ready-failed =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल आपके फ़ोल्डरों में नहीं रखी जा सकी। जिसने स्थानांतरण किया, उनसे इसे फिर से ले जाने को कहें।
       *[other] { $count } फ़ाइलें आपके फ़ोल्डरों में नहीं रखी जा सकीं। जिसने स्थानांतरण किया, उनसे इन्हें फिर से ले जाने को कहें।
    }
ready-done = हो गया
move-status-kept = सुरक्षित
move-wait-eyebrow = आपका स्थानांतरण
move-wait-scanning-title = आपकी फ़ाइलें ढूँढी जा रही हैं
move-wait-scanning-body = आपका पुराना लैपटॉप आपके फ़ोल्डर देख रहा है। वह सिर्फ़ उन्हें पढ़ता है: कुछ भी खोला, बदला या हटाया नहीं जाता।
move-found = अब तक मिला: आपकी चीज़ों में से { $size }।
move-wait-approval-title = नए लैपटॉप का इंतज़ार है
move-wait-approval-body = आपकी फ़ाइलें तैयार हैं। नए लैपटॉप पर योजना देखें और शुरू करने के लिए उसे मंज़ूर करें।
move-wait-list-title = पुराने लैपटॉप का इंतज़ार है
move-wait-list-body = आपका पुराना लैपटॉप आपकी चीज़ें खोज रहा है। फिर वहीं चुना जाता है कि क्या ले जाना है। यह हो जाने पर सूची यहाँ दिखेगी, ताकि आप उसे स्वीकार करें। इसमें कुछ मिनट लग सकते हैं।
move-wait-starting-title = स्थानांतरण शुरू हो रहा है
move-wait-starting-body = योजना मंज़ूर हो गई है। कॉपी करना कुछ ही पल में शुरू होगा।
move-wait-admin-title = एडमिनिस्ट्रेटर का इंतज़ार है
move-wait-admin-body = यह लैपटॉप दूसरे लोगों की चीज़ें भी ले जाने के लिए एडमिनिस्ट्रेटर की मंज़ूरी माँग रहा है। लैपटॉप ने जो विंडो खोली है, उसमें जवाब दें।
move-others-not-approved = एडमिनिस्ट्रेटर ने मंज़ूरी नहीं दी, इसलिए इस बार सिर्फ़ आपकी चीज़ें जाएँगी। दूसरे लोगों की चीज़ें अपनी जगह रहेंगी।
move-others-failed = PCTwin दूसरे लोगों की चीज़ें ले जाने के लिए ज़रूरी हिस्सा शुरू नहीं कर सका, इसलिए इस बार सिर्फ़ आपकी चीज़ें जाएँगी। उनकी कोई चीज़ नहीं बदली।
move-others-not-here = इस संस्करण में इस लैपटॉप से दूसरे लोगों की चीज़ें ले जाना अभी तैयार नहीं है, इसलिए सिर्फ़ आपकी चीज़ें जाएँगी।
move-others-allow-in-settings = पहले, एडमिनिस्ट्रेटर को अभी खुले सिस्टम सेटिंग्स पेज में PCTwin को अनुमति देनी होगी। इस बार सिर्फ़ आपकी चीज़ें जाएँगी; PCTwin को अनुमति मिलने के बाद, आपकी अगली बार की चीज़ों के साथ दूसरे लोगों की चीज़ें भी जा सकेंगी।
approve-prompt = PCTwin नए लैपटॉप पर ले जाने के लिए { $names } की फ़ाइलें पढ़ेगा। इस लैपटॉप का एडमिनिस्ट्रेटर सिर्फ़ इसी बार के लिए मंज़ूरी देता है।
who-here-title = कौन जा रहा है?
who-here-body = आपकी चीज़ें हमेशा जाती हैं। इस लैपटॉप पर किसी और को चुनें जिसकी चीज़ें भी जानी चाहिए। PCTwin आपको उनकी फ़ाइलें कभी नहीं दिखाता।
who-here-you = { $name } (आप)
who-here-you-hint = हमेशा जाता है
who-here-approval = किसी और की चीज़ें ले जाने के लिए एडमिनिस्ट्रेटर की मंज़ूरी चाहिए, जिन्हें भी आप चुनें उन सबके लिए एक बार। आपका लैपटॉप अपनी विंडो में पूछेगा।
who-here-not-here = इस संस्करण में इस लैपटॉप से दूसरे लोगों की चीज़ें ले जाना अभी तैयार नहीं है। सिर्फ़ आपकी चीज़ें जाएँगी।
place-title = उनकी चीज़ें कहाँ जाएँगी?
place-body = पुराने लैपटॉप से दूसरे लोगों की चीज़ें भी आई हैं। हर व्यक्ति के लिए इस लैपटॉप का कोई खाता चुनें, उनके लिए नया खाता, या उन्हें छोड़ दें। आप उनकी फ़ाइलें कभी नहीं देखते।
place-size =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल, { $size }
       *[other] { $count } फ़ाइलें, { $size }
    }
place-select = { $name } की चीज़ें कहाँ जाएँगी
place-note = नए खातों के लिए, और उनके साइन इन करने तक उनकी चीज़ें सुरक्षित रखने के लिए, योजना मंज़ूर करने के बाद एडमिनिस्ट्रेटर की मंज़ूरी एक बार चाहिए। हर नए खाते का अस्थायी पासवर्ड लैपटॉप की अपनी विंडो में एक बार दिखता है।
place-todo =
    { NUMBER($count) ->
        [one] { $count } और व्यक्ति की चीज़ों की जगह चुनें।
       *[other] { $count } और लोगों की चीज़ों की जगह चुनें।
    }
fda-title = PCTwin को कुछ फ़ोल्डर पढ़ने की अनुमति चाहिए
fda-body =
    { NUMBER($count) ->
        [one] Mac की निजता सेटिंग की वजह से आपकी { $count } जगह पढ़ी नहीं जा सकी। PCTwin को एक्सेस दें, फिर दोबारा जाँचें, ताकि आपकी कोई फ़ाइल न छूटे।
       *[other] Mac की निजता सेटिंग की वजह से आपकी { $count } जगहें पढ़ी नहीं जा सकीं। PCTwin को एक्सेस दें, फिर दोबारा जाँचें, ताकि आपकी कोई फ़ाइल न छूटे।
    }
fda-step-1 = सिस्टम सेटिंग्स खोलें, फिर गोपनीयता और सुरक्षा।
fda-step-2 = फ़ुल डिस्क एक्सेस चुनें।
fda-step-3 = PCTwin चालू करें। आपका Mac पासवर्ड माँग सकता है।
fda-open = ये सेटिंग्स खोलें
fda-note = PCTwin आपके फ़ोल्डर सिर्फ़ कॉपी करने के लिए पढ़ता है। यह इस लैपटॉप पर कभी कुछ नहीं बदलता।
fda-again = फिर से जाँचें
fda-without = इनके बिना आगे बढ़ें
password-title = PCTwin: एक नया खाता
password-body =
    अब { $name } का इस लैपटॉप पर खाता है।
    
    साइन-इन नाम: { $account }
    अस्थायी पासवर्ड: { $password }
    
    ये { $name } को दें। पहली बार साइन इन करने पर वे अपना पासवर्ड चुनेंगे। इसे अभी लिख लें: PCTwin इसे नहीं रखता।
move-ended-eyebrow = आपका स्थानांतरण
move-ended-kept-title = पहले से कॉपी हुआ
move-ended-kept-body = इससे पहले जो कुछ कॉपी और जाँचा गया, वह नए लैपटॉप पर रहता है।
move-ended-old-title = आपका पुराना लैपटॉप
move-ended-old-body = उस पर कुछ भी बदला या हटाया नहीं गया।
move-ended-old-badge = बिना बदलाव
move-ended-next = फिर से कोशिश करने के लिए लैपटॉप दोबारा जोड़ें। पहले से कॉपी और जाँची गई फ़ाइलें पहचानी जाती हैं और दोबारा कॉपी नहीं होतीं।
move-lost-title = आपके दूसरे लैपटॉप से कनेक्शन टूट गया
move-lost-body = लैपटॉप एक-दूसरे तक नहीं पहुँच पाए, इसलिए स्थानांतरण आगे नहीं बढ़ सका। देखें कि दोनों चालू हैं, जागे हुए हैं और पास-पास हैं।
move-not-approved-title = स्थानांतरण शुरू नहीं हुआ
move-not-approved-old = नए लैपटॉप पर योजना मंज़ूर नहीं हुई, इसलिए कुछ भी कॉपी नहीं हुआ।
move-not-approved-new = आपने योजना मंज़ूर नहीं की, इसलिए कुछ भी कॉपी नहीं हुआ।
move-failed-title = एक त्रुटि की वजह से स्थानांतरण रुक गया
move-failed-body = स्थानांतरण आगे नहीं बढ़ सका। अगर ऐसा फिर हो, तो नीचे ज़्यादा जानकारी में बताया गया है कि क्या गड़बड़ हुई।
move-peer-system-windows = Windows
move-peer-system-mac = Mac
move-peer-system-linux = Linux
move-peer-system-other = कोई दूसरा सिस्टम
move-update-here-title = इस लैपटॉप पर PCTwin अपडेट करें
move-update-here-body = इस लैपटॉप पर PCTwin दूसरे लैपटॉप से पुराना है, इसलिए दोनों मिलकर फ़ाइलें नहीं ले जा सकते। यहाँ PCTwin अपडेट करें, फिर से शुरू करें। कुछ भी कॉपी नहीं हुआ।
move-update-there-title = दूसरे लैपटॉप पर PCTwin अपडेट करें
move-update-there-body = दूसरे लैपटॉप पर PCTwin इस लैपटॉप से पुराना है, इसलिए दोनों मिलकर फ़ाइलें नहीं ले जा सकते। वहाँ PCTwin अपडेट करें, फिर से शुरू करें। कुछ भी कॉपी नहीं हुआ।
move-refused-title = स्थानांतरण शुरू नहीं हो सका
move-refused-not-paired = आपके लैपटॉप अब जुड़े नहीं हैं। स्थानांतरण शुरू करने के लिए उन्हें फिर से जोड़ें।
move-refused-busy = इस लैपटॉप पर पहले से एक स्थानांतरण चल रहा है।
move-refused-not-moving = इस लैपटॉप पर कोई स्थानांतरण नहीं चल रहा है।
move-refused-plan-changed = मंज़ूरी से पहले योजना बदल गई। नई योजना देखने के लिए लैपटॉप फिर से जोड़ें।
move-refused-failed = इस लैपटॉप के फ़ोल्डर स्थानांतरण के लिए तैयार नहीं हो सके। फिर से कोशिश करने के लिए लैपटॉप दोबारा जोड़ें।
drive-problem-title = स्थानांतरण को आपका ध्यान चाहिए
drive-problem-body = PCTwin ने कई बार जाँचा, और { $place } में रखी कॉपियाँ मूल फ़ाइलों से मेल नहीं खाईं। ड्राइव ख़राब, भरी हुई या निकली हुई हो सकती है, इसलिए वहाँ कॉपी करना रुका है।
drive-problem-kept-title = पहले से कॉपी और जाँचा गया
drive-problem-waiting-title = { $place } में अभी कॉपी होना बाकी
drive-problem-badge = रुका हुआ
drive-problem-fix = देखें कि ड्राइव लगी है और उसमें खाली जगह है, फिर आगे बढ़ें। इससे बचने के लिए आपकी चुनी कोई चीज़ छोड़ी नहीं जाती। आप स्थानांतरण रोक भी सकते हैं।
drive-problem-carry-on = आगे बढ़ें

## R02 Your move, item by item (detailed report)

report-title = क्या ले जाया गया
report-body = कोई समूह चुनें और देखें कि उसमें क्या है। जिन चीज़ों पर आपको ध्यान देना है, वे सबसे ऊपर हैं।
report-needs-title = { $count ->
        [one] आपको देखना है: { $count }
       *[other] आपको देखना है: { $count }
    }
report-group-photos = फ़ोटो
report-group-videos = वीडियो
report-group-music = संगीत
report-group-documents = दस्तावेज़
report-group-other = अन्य फ़ाइलें
report-group-size = { $files ->
        [one] { $files } फ़ाइल
       *[other] { $files } फ़ाइलें
    } · { $size }
report-group-done = सब कॉपी और जाँच लिया गया
report-group-some = { $total } में से { $done } कॉपी और जाँच लिए गए
report-group-left = { $count ->
        [one] { $count } नहीं ले जाया गया: आपने इसे छोड़ा
       *[other] { $count } नहीं ले जाए गए: आपने इन्हें छोड़ा
    }
report-filter-label = दिखाएं
report-col-item = आइटम
report-col-result = नतीजा
report-col-details = जाँच या अगला कदम
report-from = कहाँ से: { $path }
report-to = कहाँ: { $path }
report-none = इस नतीजे वाला कोई आइटम नहीं है।
report-more-details = और जानकारी
# Added 23 October 2026: what the report says beside a line's result (changed since the scan).
report-note-changed-since-scan = स्कैन के बाद बदला: इसे अभी जैसा है, वैसा ही कॉपी किया गया।
report-note-no-longer-there = अब वहाँ नहीं है: भेजे जाने से पहले इसे पुराने लैपटॉप पर हटाया, नाम बदला या मिटाया गया।
report-note-time-not-kept = तारीख़ नहीं रखी गई: यह ड्राइव इसके आख़िरी बदलाव की तारीख़ नहीं रख सकती, इसलिए यह कॉपी होने की तारीख़ दिखाती है।

# Evidence: how a copied item was checked.
evidence-read-back = ड्राइव से वापस पढ़कर जाँचा गया।
evidence-memory = इस लैपटॉप की मेमोरी से जाँचा गया, ड्राइव से नहीं।
evidence-already-there = नए लैपटॉप पर पहले से था: वही फ़ाइल वहाँ थी, इसलिए इसे दोबारा कॉपी नहीं किया गया।
evidence-recovered = लैपटॉप के फिर से चालू होने से पहले जाँचा गया, और बाद में दोबारा जाँचा गया।
evidence-new-laptop = नए लैपटॉप ने जाँचा।
evidence-memory-explain = कॉपी को वापस पढ़ा गया और वह मेल खाई, लेकिन इस लैपटॉप की मेमोरी के ज़रिए, इसलिए हो सकता है कि जो पढ़ा गया वह ड्राइव के बजाय मेमोरी से आया हो। ड्राइव में ही कोई ख़राबी हो तो वह यहाँ शायद न दिखे, इसलिए फ़ाइल को एक बार खोलकर देखना अच्छी जाँच है।
evidence-not-checked = जाँचा नहीं गया, क्योंकि यह पूरा नहीं हुआ।
# Machine-translated 16 October 2026.
why-memory-cannot-skip-cache = इस ड्राइव ने PCTwin को इसे सीधे पढ़ने नहीं दिया, इसलिए जाँच इस लैपटॉप की मेमोरी के ज़रिए हुई। कुछ नेटवर्क ड्राइव और USB स्टिक ऐसे ही काम करती हैं।
why-memory-not-shown = यह ड्राइव यह नहीं दिखा सकी कि PCTwin का पढ़ना सच में उस तक पहुँचा, इसलिए इस जाँच को इस लैपटॉप की मेमोरी से हुई जाँच माना गया।
why-memory-no-way = इस लैपटॉप के सिस्टम के पास इस ड्राइव को सीधे पढ़ने का कोई तरीका नहीं है, इसलिए जाँच उसकी मेमोरी के ज़रिए हुई।
why-memory-stored-specially = यह फ़ाइल ख़ास तरीके से रखी गई है, जैसे कंप्रेस या एन्क्रिप्ट करके, इसलिए सिस्टम इसे मेमोरी के ज़रिए पढ़ता है।
why-memory-too-small = यह फ़ाइल बहुत छोटी है, इसलिए ड्राइव इसे अपने ही रिकॉर्ड के साथ रखती है और सिस्टम इसे मेमोरी के ज़रिए पढ़ता है।

# Reasons: why an item did not finish. Each says what happened, then what to do.
reason-not-enough-space = नए लैपटॉप पर स्टोरेज ख़त्म हो गया। उस पर स्टोरेज खाली करें, फिर इस आइटम को शामिल करके नया स्थानांतरण शुरू करें।
reason-changed-during-move = कॉपी होते समय यह फ़ाइल पुराने लैपटॉप पर बदलती रही। इसे इस्तेमाल कर रहा कोई भी ऐप बंद करें, फिर इसे शामिल करके नया स्थानांतरण शुरू करें।
reason-unreadable = पुराने लैपटॉप ने PCTwin को यह आइटम पढ़ने नहीं दिया। जाँचें कि यह पुराने लैपटॉप पर खुलता है, फिर इसे शामिल करके नया स्थानांतरण शुरू करें।
reason-not-verified = कॉपी मूल से मेल नहीं खाई, इसलिए उसे नहीं रखा गया। इसे शामिल करके नया स्थानांतरण शुरू करें। अगर फिर से ऐसा हो, तो हो सकता है मूल फ़ाइल ख़राब हो।
reason-refused-by-safety-check = नए लैपटॉप की सुरक्षा जाँच ने इस आइटम को उसके नाम, जगह या साइज़ की वजह से नहीं लिया। मूल अब भी पुराने लैपटॉप पर है, इसलिए ज़रूरत हो तो इसे किसी और तरीके से कॉपी किया जा सकता है।
reason-name-not-allowed = यह नाम नए लैपटॉप पर इस्तेमाल नहीं हो सकता, और कोई सुरक्षित नया नाम बनाना मुमकिन नहीं था। पुराने लैपटॉप पर इसका नाम बदलें, फिर इसे शामिल करके नया स्थानांतरण शुरू करें।
reason-install-failed = ऐप इंस्टॉल नहीं हुआ। इसे इसके बनाने वाले की वेबसाइट से इंस्टॉल करें।
reason-no-equivalent = नए लैपटॉप के सिस्टम के लिए इस ऐप का कोई वर्शन नहीं है। कोई मिलता-जुलता ऐप ढूंढें जो वहाँ चलता हो।
reason-drive-locked = जिस ड्राइव पर यह जाना है, वह लॉक है। उसे अनलॉक करें (जैसे उसके BitLocker पासवर्ड या रिकवरी की से), फिर इसे दोबारा ले जाएँ।
reason-sign-in = यह अपनी जगह पर है, लेकिन आपको साइन इन करना होगा। इसे खोलें और साइन इन करें।
reason-activation = ऐप अपनी जगह पर है, लेकिन नए लैपटॉप पर इसे इसका लाइसेंस चाहिए। इसे खोलें और अपने लाइसेंस या अकाउंट से इसे चालू करें।
reason-permission = नए लैपटॉप को पहले अनुमति देनी होगी। आइटम खोलें और पूछे जाने पर “अनुमति दें” चुनें।
reason-online-only = यह फ़ाइल सिर्फ़ ऑनलाइन रखी है, इसलिए कॉपी करने के लिए पुराने लैपटॉप पर नहीं थी। इसे नए लैपटॉप पर उसी ऐप से डाउनलोड करें जिसमें यह ऑनलाइन रखी है।
reason-manual-install = यह ऐप इसके बनाने वाले की वेबसाइट से ही आना चाहिए। इसे वहीं से डाउनलोड और इंस्टॉल करें।
reason-restart = पूरा करने के लिए नए लैपटॉप को रीस्टार्ट करना होगा। तैयार होने पर इसे रीस्टार्ट करें।
reason-finish-by-hand = PCTwin यह कदम आपके लिए नहीं कर सकता। इस रिपोर्ट में इसके लिए दिखाए गए कदम अपनाएं।
reason-parts-missing = इस फ़ोल्डर की कुछ फ़ाइलें नहीं पहुँचीं, इसलिए नए लैपटॉप पर यह पूरा नहीं है। वे इस रिपोर्ट में कारण के साथ दी गई हैं। उन्हें फिर से भेजें, या फ़ोल्डर को किसी और तरीके से कॉपी करें।
reason-moved-file-missing = यह सेटिंग ऐसी फ़ाइल या फ़ोल्डर की ओर इशारा करती है जो नए लैपटॉप पर उस नाम से नहीं है जो स्थानांतरण ने उसे दिया, इसलिए PCTwin ने सेटिंग जैसी थी वैसी छोड़ दी। ऐप खोलें और फ़ाइल फिर से चुनें।

## Item results (every item ends with one of these)

status-copied = कॉपी हो गया
status-reinstalled = फिर से इंस्टॉल हुआ
status-converted = कन्वर्ट हुआ
status-needs-action = कार्रवाई ज़रूरी
status-skipped-by-user = आपने छोड़ा
status-failed = विफल

## Screens: words the copy deck did not cover (same voice)

# $low and $high are whole minutes, a measured range. The result fills "About { $range } left".
progress-minutes = { $low } से { $high } मिनट
# Machine-translated 16 October 2026.
progress-minutes-one =
    { NUMBER($minutes) ->
        [one] { $minutes } मिनट
       *[other] { $minutes } मिनट
    }
# $low and $high are numbers and $unit is already formatted, for example "MB/s". The result fills "{ $speed }, measured".
progress-speed-range = { $low } से { $high } { $unit }
progress-count-failed =
    { NUMBER($count) ->
        [one] { $count } आइटम कॉपी नहीं हो सका
       *[other] { $count } आइटम कॉपी नहीं हो सके
    }
progress-queue-empty = अभी तक कुछ भी शुरू नहीं हुआ है।
progress-approved-job = मंज़ूर किया गया स्थानांतरण
progress-route-old = पुराना लैपटॉप
progress-route-new = नया लैपटॉप
progress-panel-pausing = रोका जा रहा है
outcome-metric-size = कॉपी और जाँचा गया
outcome-metric-action = पर आपकी कार्रवाई ज़रूरी
progress-percent-unknown = PCTwin अभी इस स्थानांतरण का साइज़ पता कर रहा है।
mini-title-send-paused = भेजना रुका हुआ है
cancel-ack-hint = बंद करने का बटन चालू करने के लिए ऊपर वाले बॉक्स पर टिक करें।
progress-state-placing = जगह पर रखा जा रहा है
progress-state-existing = पहले से मौजूद
outcome-stopped-title = आपका स्थानांतरण बंद कर दिया गया
outcome-stopped-body = स्थानांतरण आपने बंद किया। उससे पहले जो कॉपी और जाँचा गया था, वह रखा गया है। बाकी कॉपी नहीं हुआ।
outcome-broke-title = आपका स्थानांतरण समय से पहले खत्म हो गया
outcome-broke-body = लैपटॉप के बीच कुछ गड़बड़ हुई, इसलिए स्थानांतरण आगे नहीं चल सका। उससे पहले जो कॉपी और जाँचा गया था, वह रखा गया है। बाकी कॉपी नहीं हुआ।
# $done and $total are counts of items: what was copied and checked, and everything the person approved.
outcome-files-summary = { $total } में से { $done } आइटम कॉपी और जाँचे गए।
outcome-files-memory =
    { NUMBER($count) ->
        [one] { $count } इस लैपटॉप की मेमोरी से जाँचा गया, ड्राइव से नहीं।
       *[other] { $count } इस लैपटॉप की मेमोरी से जाँचे गए, ड्राइव से नहीं।
    }
report-show-more = और दिखाएं
# $shown and $total are counts of items.
report-showing = { $total } में से { $shown } दिखाए जा रहे हैं

## Component kit

dialog-close = बंद करें
toast-dismiss = हटाएं
code-label = छह अंकों का कोड

## Shared Quick Send words

qs-eyebrow = क्विक सेंड
qs-eyebrow-receiving = आपके लैपटॉप पर प्राप्त हो रहा है
qs-label-this = यह लैपटॉप
qs-label-other = दूसरा लैपटॉप
qs-pick-failed = चुनने वाली विंडो नहीं खुल सकी। फिर से कोशिश करें।
qs-handed =
    { NUMBER($count) ->
        [one] भेजने के लिए 1 फ़ाइल या फ़ोल्डर PCTwin को दिया गया। जारी रखने से पहले सूची जाँच लें।
       *[other] भेजने के लिए { $count } फ़ाइलें या फ़ोल्डर PCTwin को दिए गए। जारी रखने से पहले सूची जाँच लें।
    }
qs-pick-too-many = ये एक बार भेजने में जा सकने वाली फ़ाइलों से ज़्यादा हैं। कुछ फ़ोल्डरों से टिक हटाएँ, या इन्हें हिस्सों में भेजें।
qs-problem-lost-title = लैपटॉप के बीच कनेक्शन टूट गया
qs-problem-failed-title = इस बार फ़ाइलें नहीं भेजी जा सकीं
qs-problem-body = जो पहले से मिल चुका और जाँचा जा चुका है, वह अपनी जगह रहता है। फिर से कोशिश करने के लिए लैपटॉप दोबारा जोड़ें।
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = { $from } से { $to } तक
qs-route-sender = भेजा जा रहा है
qs-route-receiver = प्राप्त कर रहा है · वही Wi-Fi
qs-tally =
    { NUMBER($count) ->
        [one] { $count } चुना गया आइटम · { $size }
       *[other] { $count } चुने गए आइटम · { $size }
    }
qs-tally-badge = आस-पास भेजना

## Q01 Quick Send home

qs-home-title = आस-पास के किसी लैपटॉप पर फ़ाइलें भेजें
qs-home-body = या किसी लैपटॉप से फ़ाइलें पाएँ।
qs-send-title = फ़ाइलें भेजें
qs-send-body = अपने लैपटॉप पर फ़ाइलें या फ़ोल्डर चुनें।
qs-receive-title = फ़ाइलें प्राप्त करें
qs-receive-body = पास के किसी लैपटॉप को आपके लैपटॉप पर फ़ाइलें भेजने दें।
qs-home-action = फ़ाइलें चुनें
qs-receiving = प्राप्त करने के लिए तैयार। पास के लैपटॉप आपके लैपटॉप को { $label } नाम से देखते हैं।
qs-receive-stop = प्राप्त करना बंद करें
qs-no-nearby = पास में कोई लैपटॉप नहीं मिला। भेजने वाले लैपटॉप पर PCTwin खोलें और दोनों को एक ही Wi-Fi पर रखें।

## Q02 Choose files and folders

qs-choose-title = क्या भेजना है?
qs-choose-body = सिर्फ़ वही फ़ाइलें और फ़ोल्डर चुनें जिन्हें शेयर करना है।
qs-drop =
    .aria-label = फ़ाइलें और फ़ोल्डर छोड़ने की जगह
qs-drop-title = फ़ाइलें या फ़ोल्डर यहाँ खींचकर छोड़ें
qs-drop-body = या “फ़ाइलें शामिल करें” या “फ़ोल्डर शामिल करें” से इन्हें शामिल करें।
qs-drop-active = शामिल करने के लिए छोड़ दें।
qs-add-files = फ़ाइलें शामिल करें
qs-add-folders = फ़ोल्डर शामिल करें
qs-remove =
    .aria-label = { $name } हटाएं
# $kind is the file type, for example "PDF document" or "8 photos".
qs-file-detail = { $size } · { $kind }
qs-empty = भेजने के लिए फ़ाइलें या फ़ोल्डर शामिल करें।
qs-online-only = कुछ फ़ाइलें सिर्फ़ ऑनलाइन हैं। पहले उन्हें अपने लैपटॉप पर उपलब्ध करें, फिर भेजें।
qs-protected = PCTwin { $name } को नहीं पढ़ सकता। पहुँच की अनुमति दें, या इसे सूची से हटाएं।
qs-allow = पहुँच की अनुमति दें
qs-choose-continue = जारी रखें

## Q12 Send progress

qs-sending-title = आपकी फ़ाइलें भेजी जा रही हैं
qs-sending-body = कॉपियाँ सीधे पाने वाले लैपटॉप पर जाती हैं।
qs-sending-progress =
    .aria-label = फ़ाइलें भेजी जा रही हैं
qs-sending-current = अभी भेजा जा रहा है: { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = { $speed }, मापा गया
qs-step-copying = कॉपी हो रहा है
qs-step-copying-body =
    { NUMBER($done) ->
        [one] { $done } फ़ाइल भेजी गई।
       *[other] { $done } फ़ाइलें भेजी गईं।
    }
qs-step-checking = जाँच
qs-step-checking-body = हर फ़ाइल की तुलना मूल फ़ाइल से की जाती है।
qs-sending-paused = रुका हुआ। जो फ़ाइलें प्राप्त और जाँची जा चुकी हैं, वे रखी गई हैं।
qs-sending-disconnected = { $label } से कनेक्शन टूट गया। जो फ़ाइलें जाँची जा चुकी हैं, वे रखी गई हैं। अपने-आप फिर से कोशिश हो रही है…
qs-sending-needs-action =
    { NUMBER($count) ->
        [one] { $count } आइटम पर आपको कुछ करना है।
       *[other] { $count } आइटम पर आपको कुछ करना है।
    }
qs-pause = भेजना रोकें
qs-resume = फिर से शुरू करें
qs-compact = कम दिखाएँ
qs-stop = यह भेजना बंद करें
qs-sending-complete-title = भेजना पूरा हुआ

## Q20 Stop this send? (and decline)

qs-stop-title = यह भेजना बंद करें?
qs-stop-body = बंद करने से पहले देखें कि क्या पहुँच चुका है।
qs-stop-arrived-title = पूरी पहुँची फ़ाइलें
qs-stop-arrived-body = आपके चुने फ़ोल्डर में रहती हैं।
qs-stop-rest-title = अभी न भेजी गई फ़ाइलें
qs-stop-rest-body = बंद करने के बाद कुछ भी नया नहीं भेजा जाता।
qs-stop-partial-title = आधी पहुँची फ़ाइलें
qs-stop-partial-body = कभी उनके असली नाम से नहीं सहेजी जातीं, ताकि उन्हें पूरी कॉपी न समझा जाए।
qs-stop-status-keep = रखें
qs-stop-status-stop = बंद
qs-stop-status-review = देखें
qs-stop-ack = यह भेजना बंद करें
qs-stop-ack-hint = जो फ़ाइलें पूरी पहुँच गईं, वे हटाई नहीं जातीं।
qs-stop-keep = भेजना जारी रखें

## F08 Your moves, kept on this laptop (history)

history-title = आपका स्थानांतरण इतिहास
history-body = कोई रिपोर्ट फिर से खोलें, या सहेजा गया स्थानांतरण आगे बढ़ाएं।
history-row-move = { $from } → { $to }
history-status-finished = पूरा हुआ
history-status-steps-left =
    { NUMBER($count) ->
        [one] पूरा हुआ, आपके लिए { $count } चरण बाकी
       *[other] पूरा हुआ, आपके लिए { $count } चरण बाकी
    }
history-status-paused = रुका हुआ
history-status-stopped = रोका गया
history-open-report = रिपोर्ट
history-undo = पहले जैसा करें
history-continue = आगे बढ़ें
history-export = इतिहास फ़ाइल में सहेजें
history-exported = इतिहास { $path } में सहेजा गया।
history-export-failed = इतिहास वहाँ सहेजा नहीं जा सका। कोई दूसरा फ़ोल्डर चुनें और फिर से कोशिश करें।
history-note = यह इतिहास इसी लैपटॉप पर रहता है। जब तक आप इसे फ़ाइल में सहेजकर ख़ुद शेयर न करें, कुछ भी कहीं नहीं भेजा जाता।
history-empty = आपका पहला स्थानांतरण और उसकी रिपोर्ट यहाँ दिखेंगे।
history-empty-title = अभी तक कोई स्थानांतरण दर्ज नहीं
history-start = नया स्थानांतरण शुरू करें

## Quick Send: words the mockups do not have (added while building; same voice)

qs-soon-note = इस वर्शन में क्विक सेंड अभी चालू नहीं है। कुछ भी भेजा या प्राप्त नहीं किया गया है।
qs-tally-counting =
    { NUMBER($count) ->
        [one] { $count } चुना गया आइटम · साइज़ अभी गिना जा रहा है
       *[other] { $count } चुने गए आइटम · साइज़ अभी गिना जा रहा है
    }
qs-gone = { $name } अब यहां नहीं है। शायद इसे हटा दिया गया या दूसरी जगह ले जाया गया। इसका टिक हटाएं, या इसे वापस रखकर फिर से शामिल करें।
qs-choose-why-problems = टिक किए गए कुछ आइटम अभी भेजे नहीं जा सकते। आगे बढ़ने के लिए उनका टिक हटाएं, या बताए गए तरीके से उन्हें ठीक करें।
qs-sending-card = फ़ाइलें भेजी जा रही हैं
qs-progress-of = कुल { $total }
history-open-name = { $what }: { $title }

## R05 वापस लेना: हर ड्राइव पर वापस लेने से क्या होगा, और हर कॉपी का क्या हुआ

undo-here-removes = वापस लेने पर PCTwin की बनाई वे कॉपी हट जाती हैं जिन्हें आपने नहीं बदला। आपने जो कुछ बदला है, वह रखा जाता है।
undo-here-to-trash = वापस लेने पर PCTwin की बनाई वे कॉपी ट्रैश में चली जाती हैं जिन्हें आपने नहीं बदला। आपने जो कुछ बदला है, वह रखा जाता है, और कुछ भी मिटाया नहीं जाता।
undo-here-keeps-fat = वापस लेने पर { $drive } पर हर कॉपी रखी जाएगी। यह ड्राइव FAT या exFAT इस्तेमाल करती है, जो फ़ाइलों को अलग पहचानने वाले नंबर दोबारा इस्तेमाल करता है, इसलिए PCTwin पक्का नहीं कह सकता कि कौन-सी फ़ाइलें उसकी हैं। आप उन्हें ख़ुद मिटा सकते हैं।
undo-here-keeps-other = वापस लेने पर { $drive } पर हर कॉपी रखी जाएगी। इस तरह की ड्राइव पर PCTwin फ़ाइलों को सुरक्षित रूप से न हटा सकता है, न खिसका सकता है। आप उन्हें ख़ुद मिटा सकते हैं।
undo-here-more = और जानकारी
undo-here-more-trash = Mac पर PCTwin यह पक्का नहीं कर सकता कि कोई और ऐप अब भी किसी फ़ाइल में नहीं लिख रहा है, इसलिए PCTwin कभी कोई फ़ाइल नहीं मिटाता। वह हर कॉपी को उसी ड्राइव के ट्रैश में डाल देता है। आप उसे ट्रैश से वापस ला सकते हैं।
undo-here-more-fat = FAT और exFAT ड्राइव पर किसी नई फ़ाइल को वह नंबर मिल सकता है जो किसी मिटाई गई फ़ाइल का था। PCTwin सिर्फ़ उन्हीं फ़ाइलों पर कुछ करता है जिनके बारे में वह साबित कर सके कि वे उसकी हैं और बदली नहीं गईं, इसलिए ये फ़ाइलें रखी जाती हैं।
undo-done-trashed = “{ $name }” नाम से ट्रैश में डाल दी गई। आप उसे ट्रैश से वापस ला सकते हैं।
undo-kept-fat = रखी गई। यह ड्राइव (FAT या exFAT) फ़ाइलों को अलग पहचानने वाले नंबर दोबारा इस्तेमाल करती है, इसलिए PCTwin पक्का नहीं कह सकता कि यह उसकी कॉपी है। अगर आपको इसकी ज़रूरत नहीं है, तो आप इसे ख़ुद मिटा सकते हैं।
undo-kept-no-trash = रखी गई। PCTwin इस कॉपी को उसकी ड्राइव के ट्रैश में नहीं डाल सका। अगर आपको इसकी ज़रूरत नहीं है, तो आप इसे ख़ुद मिटा सकते हैं।

## Quick Send (26 October 2026): pairing in Quick Send's words, where received files go, and the Done page

qs-pair-old-eyebrow = भेजा जा रहा है
qs-pair-old-body = पाने वाले लैपटॉप पर “फ़ाइलें प्राप्त करें” चुनें, फिर यह कोड डालें।
qs-pair-old-waiting = पाने वाले लैपटॉप का इंतज़ार है…
qs-pair-old-label-hint = पाने वाला लैपटॉप आपके लैपटॉप को इसी नाम से ढूँढेगा।
qs-pair-old-steps-title = पाने वाले लैपटॉप पर
qs-pair-old-step-1 = PCTwin खोलें, “क्विक सेंड” चुनें, फिर “फ़ाइलें प्राप्त करें”।
qs-pair-old-left = नंबर चुने जाने से पहले पाने वाले लैपटॉप ने जोड़ना बंद कर दिया, इसलिए कुछ भी शेयर नहीं हुआ। यह नया कोड है।
qs-pair-network-hint = वह नेटवर्क चुनें जिस पर पाने वाला लैपटॉप है।
qs-pair-choose-body = पाने वाले लैपटॉप को देखें। जो नंबर वह दिखाता है, उसे चुनें।
qs-pair-choose-renewed = नए नंबर दिख रहे हैं। पाने वाले लैपटॉप वाला नंबर चुनें।
qs-pair-choose-note = अगर पाने वाला लैपटॉप कोई नंबर नहीं दिखाता, या इनमें से कोई उससे मेल नहीं खाता, तो “इनमें से कोई मेल नहीं खाता” चुनें।
qs-pair-heads-up-firewall = Windows पूछ सकता है कि क्या PCTwin कनेक्शन स्वीकार कर सकता है। “अनुमति दें” चुनें, ताकि पाने वाला लैपटॉप आपके लैपटॉप तक पहुँच सके।
qs-pair-new-eyebrow = प्राप्त हो रहा है
qs-pair-new-searching-title = भेजने वाले लैपटॉप को खोजा जा रहा है…
qs-pair-new-searching-body = भेजने वाले लैपटॉप पर PCTwin खुला रखें, उसी Wi-Fi पर जिस पर आपका लैपटॉप है।
qs-pair-new-search-failed-title = भेजने वाले लैपटॉप को खोजा नहीं जा सका
qs-pair-new-list-title = कौन-सा लैपटॉप भेज रहा है?
qs-pair-new-list-body = वह नाम चुनें जो भेजने वाला लैपटॉप अपनी स्क्रीन पर दिखाता है।
qs-pair-new-same-label = पास के किसी दूसरे लैपटॉप का भी यही नाम है। भेजने वाले लैपटॉप की स्क्रीन देखें।
qs-pair-new-none-title = भेजने वाला कोई लैपटॉप नहीं मिला
qs-pair-new-none-body = जाँचें कि भेजने वाले लैपटॉप पर PCTwin खुला है और कोड दिखा रहा है, और दोनों लैपटॉप एक ही Wi-Fi पर हैं। फिर से खोजें।
qs-pair-new-code-hint = इसे { $label } की स्क्रीन से पढ़ें। यह हर मिनट बदलता है।
qs-pair-new-number-title = { $label } पर यह नंबर चुनें
qs-pair-new-number-body = { $label } पर तीन नंबर दिखते हैं। वहाँ यह नंबर चुनें।
qs-pair-new-number-waiting = { $label } का इंतज़ार है…
qs-pairing-code-invalid = कोड में छह अंक होने चाहिए। { $label } की स्क्रीन देखें और कोड फिर से लिखें।
qs-pairing-code-wrong = कोड मेल नहीं खाया। { $label } की स्क्रीन देखें और फिर से कोशिश करें।
qs-pairing-code-expired = कोड बदल गया है। { $label } की स्क्रीन से नया कोड लिखें।
qs-pairing-busy = अभी { $label } से नहीं जोड़ा जा सकता: वह किसी और लैपटॉप से जुड़ रहा है। थोड़ा इंतज़ार करें, और फिर से कोशिश करें।
qs-pairing-wait = { $label } पर नया कोड तैयार हो रहा है। उसका इंतज़ार करें, और फिर नया कोड लिखें।
qs-pairing-locked = बहुत ज़्यादा गलत कोशिशों के बाद जोड़ना बंद हो गया। { $label } पर “फिर से शुरू करें” चुनें।
qs-pairing-unreachable = आपका लैपटॉप { $label } तक नहीं पहुँच सका। जाँचें कि दोनों एक ही Wi-Fi पर हैं, और फिर से कोशिश करें।
qs-pairing-wrong-number = वह नंबर पाने वाले लैपटॉप के नंबर से मेल नहीं खाया। कुछ भी शेयर नहीं हुआ। फिर से कोशिश करने के लिए पाने वाले लैपटॉप पर नया कोड लिखें।
qs-pairing-no-match = { $label } पर नंबर मेल नहीं खाए। कुछ भी शेयर नहीं हुआ। उसकी स्क्रीन से नया कोड लिखें।
qs-pairing-timed-out = समय पर कोई नंबर नहीं चुना गया। { $label } पर दिख रहे नए कोड से फिर से कोशिश करें।
qs-pairing-connection-lost-new = { $label } से कनेक्शन टूट गया। कुछ भी शेयर नहीं हुआ।
qs-pairing-choose-network = आपका लैपटॉप एक से ज़्यादा नेटवर्क पर है। वह चुनें जिस पर पाने वाला लैपटॉप है:
qs-pairing-failed-new = इस बार जोड़ना काम नहीं किया। कुछ भी शेयर नहीं हुआ। { $label } की स्क्रीन से नया कोड लिखें।
qs-folder-line = फ़ाइलें { $folder } में सहेजी जाएँगी।
qs-folder-change = बदलें
qs-folder-title = फ़ाइलें कहाँ जाएँ?
qs-folder-none = PCTwin प्राप्त फ़ाइलों वाले फ़ोल्डर का उपयोग नहीं कर सकता। फ़ाइलों के लिए कोई फ़ोल्डर चुनें, फिर जोड़ें।
qs-folder-choose = फ़ोल्डर चुनें
qs-folder-failed = PCTwin उस फ़ोल्डर में फ़ाइलें नहीं सहेज सकता। कोई दूसरा फ़ोल्डर चुनें।
qs-done-sent-title = आपकी फ़ाइलें भेज दी गईं
qs-done-here-title = फ़ाइलें मिल गईं
qs-done-partial-title = कुछ फ़ाइलें नहीं पहुँचीं
qs-done-stopped-title = यह भेजना बंद किया गया
qs-done-empty-title = कुछ भी नहीं भेजा गया
qs-done-arrived =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल · { $size } पहुँची और मूल से मेल खाई
       *[other] { $count } फ़ाइलें · { $size } पहुँचीं और मूल से मेल खाईं
    }
qs-done-arrived-count =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल पहुँची और मूल से मेल खाई
       *[other] { $count } फ़ाइलें पहुँचीं और मूल से मेल खाईं
    }
qs-done-already =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल पहले से वहाँ थी, एक जैसी, इसलिए दोबारा नहीं भेजी गई।
       *[other] { $count } फ़ाइलें पहले से वहाँ थीं, एक जैसी, इसलिए दोबारा नहीं भेजी गईं।
    }
qs-done-stopped-body = रोकने से पहले पहुँची फ़ाइलें जाँची गईं और जहाँ सहेजी गईं, वहीं रहेंगी। और कुछ नहीं भेजा गया।
qs-done-folder = { $folder } में सहेजी गईं
qs-done-open-folder = फ़ोल्डर खोलें
qs-done-dates-new =
    { NUMBER($count) ->
        [one] यह फ़ाइल अपने पहुँचने का समय दिखाती है, इसलिए यह फ़ोल्डर में सबसे ऊपर है।
       *[other] ये { $count } फ़ाइलें अपने पहुँचने का समय दिखाती हैं, इसलिए ये फ़ोल्डर में सबसे ऊपर हैं।
    }
qs-done-keep-dates = मूल तारीख़ें रखें
qs-done-dates-kept =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल को उसकी मूल तारीख़ वापस मिल गई।
       *[other] { $count } फ़ाइलों को उनकी मूल तारीख़ें वापस मिल गईं।
    }
qs-done-dates-failed = मूल तारीख़ें वापस नहीं लगाई जा सकीं। फ़ाइलें ठीक हैं और अपनी जगह पर हैं।
qs-done-open-failed = फ़ोल्डर नहीं खुल सका। हो सकता है वह हटा दिया गया हो या मिटा दिया गया हो।
qs-done-not-opened = PCTwin ने प्राप्त हुई कोई भी फ़ाइल न खोली, न चलाई।
qs-done-problems-title = जिन फ़ाइलों पर ध्यान देना है
qs-done-not-arrived = यह नहीं पहुँची।
qs-done-not-completed = यह रोकने से पहले नहीं भेजी गई।
qs-done-needs-action = यह यहाँ है, पर पूरी नहीं हुई। क्या करना है, यह और जानकारी में है।
qs-done-mark-not-kept = यह यहाँ है, पर यह ड्राइव इसे किसी दूसरे लैपटॉप से डाउनलोड की गई फ़ाइल के रूप में चिह्नित नहीं कर सकी। इसे सावधानी से खोलें।
qs-done-see-files = फ़ाइलें देखें
qs-done-renamed = { $name } नाम से सहेजी गई, क्योंकि उस नाम की फ़ाइल पहले से वहाँ थी।
qs-done-identical = एक जैसी फ़ाइल पहले से वहाँ थी, इसलिए यह दोबारा नहीं भेजी गई।
qs-done-send-missing = छूटी हुई फ़ाइलें भेजें
qs-done-send-missing-body = सिर्फ़ वे फ़ाइलें जो नहीं पहुँचीं। पहले आप लैपटॉप फिर से जोड़ेंगे।
qs-done-done = हो गया
qs-stop-now = भेजना बंद करें
qs-stop-finished = यह भेजना पहले ही पूरा हो चुका है।
qs-route-receiving = प्राप्त हो रहा है
qs-receiving-title = फ़ाइलें आ रही हैं
qs-receiving-body = कॉपियाँ सीधे { $label } से आती हैं।
qs-receiving-card = फ़ाइलें प्राप्त हो रही हैं
qs-receiving-progress =
    .aria-label = फ़ाइलें प्राप्त हो रही हैं
qs-receiving-current = अभी प्राप्त हो रही है: { $name }
qs-step-receiving-body =
    { NUMBER($done) ->
        [one] { $done } फ़ाइल पहुँची।
       *[other] { $done } फ़ाइलें पहुँचीं।
    }
qs-tally-files =
    { NUMBER($count) ->
        [one] { $count } फ़ाइल · { $size }
       *[other] { $count } फ़ाइलें · { $size }
    }
qs-receive-pause = कुछ देर रोकें
qs-label-yours = आपका लैपटॉप
qs-label-receiving = पाने वाला लैपटॉप
qs-label-sending = भेजने वाला लैपटॉप
qs-peer-receiving = पाने वाला लैपटॉप
qs-peer-sending = भेजने वाला लैपटॉप
qs-pair-old-label-title = आपके लैपटॉप का नाम
qs-pair-old-rename-label = आपके लैपटॉप का नाम
qs-pair-old-step-2 = अगर कोई सूची दिखे, तो अपने लैपटॉप का नाम चुनें।
qs-pair-old-locked-body = पाँच गलत कोड के बाद, आपके लैपटॉप को सुरक्षित रखने के लिए जोड़ना बंद हो गया। कुछ भी शेयर नहीं हुआ। अगर यह आप थे, तो नए कोड के लिए “फिर से शुरू करें” चुनें।
qs-pair-new-needs-update = उस पर PCTwin का पुराना संस्करण है। उस पर PCTwin अपडेट करें, फिर से खोजें।
qs-pairing-no-network = आपका लैपटॉप किसी Wi-Fi या तार वाले नेटवर्क पर नहीं है। उसे जोड़ें, फिर से कोशिश करें।
qs-problem-room-title = { $folder } में पर्याप्त जगह नहीं है
qs-problem-room-body = इन फ़ाइलों को { $needed } चाहिए, और वहाँ { $room } इस्तेमाल हो सकता है। कुछ भी सहेजा नहीं गया। जगह खाली करें या कोई दूसरा फ़ोल्डर चुनें, फिर से कोशिश करें।
qs-done-refused-title = पाने वाले लैपटॉप ने ये फ़ाइलें नहीं लीं
qs-done-refused-body = हो सकता है उस पर पर्याप्त जगह न हो। कुछ भी नहीं भेजा गया।

## चरण 2 की स्क्रीन (कार्य सूची 2.8): साझा हिस्से

guard-state-not-started = अभी शुरू नहीं हुआ
guard-state-preparing = तैयारी हो रही है
guard-state-awaiting-person = आपकी पसंद का इंतज़ार है
guard-state-awaiting-system = आपके सिस्टम की मंज़ूरी का इंतज़ार है
guard-state-awaiting-maker = ऐप बनाने वाले या ऑनलाइन सेवा का इंतज़ार है
guard-state-working = काम हो रहा है
guard-state-checking = जो किया गया, उसकी जाँच हो रही है
guard-state-applying = चीज़ें अपनी जगह रखी जा रही हैं
guard-state-pause-requested = मौजूदा चरण के बाद रुकेगा। अभी रुका नहीं है।
guard-state-paused = रुका हुआ
guard-state-source-unavailable = पुराना लैपटॉप या ड्राइव जुड़ा नहीं है
guard-state-blocked = पहले कुछ ठीक करना होगा
guard-state-partial = आंशिक रूप से हो गया
guard-state-retrying = फिर से कोशिश हो रही है
guard-state-reconciling = जाँच हो रही है कि क्या पूरा हुआ
guard-state-failed = यह नहीं हो पाया
guard-state-complete = हो गया
guard-state-cancelled = रोक दिया गया। जो पहले हो चुका था, वह रखा गया है।
guard-refused-unavailable = PCTwin अभी यह जाँच नहीं कर सका। कुछ नहीं किया गया। थोड़ी देर में फिर से कोशिश करें।
guard-refused-wrong-job = यह किसी दूसरे स्थानांतरण के लिए था। कुछ नहीं किया गया।
guard-refused-old-session = यह स्क्रीन पुरानी हो चुकी थी, इसलिए कुछ नहीं किया गया। अभी की सही स्थिति यह है।
guard-refused-plan-changed = आपके देखते-देखते योजना बदल गई, इसलिए कुछ नहीं किया गया। इसे फिर से देखें।
guard-refused-not-now = इस चरण पर यह नहीं हो सकता। कुछ नहीं किया गया।
guard-refused-not-shown = वह विकल्प अब इस स्क्रीन पर नहीं है। कुछ नहीं किया गया।
guard-progress-label = प्रगति
guard-progress-mark = { $percent }% पूरा
guard-progress-of = { $total } में से { $done }
guard-progress-unknown-total = अब तक { $done }। कुल संख्या अभी पता नहीं है।
guard-progress-none = अभी तक कुछ नापा नहीं गया है।
guard-and-more =
    { $count ->
        [one] और { $count } ज़्यादा, रिपोर्ट में दिए गए हैं।
       *[other] और { $count } ज़्यादा, रिपोर्ट में दिए गए हैं।
    }
guard-not-known-yet = अभी पता नहीं है
guard-free = { $free } खाली
guard-free-unknown = खाली जगह पता नहीं है
guard-items-size = { $items } आइटम, { $size }
guard-try-again = फिर से कोशिश करें
guard-pause = रोकें
guard-resume = फिर से शुरू करें
guard-check-again = फिर से जाँच करें
guard-strip-plan = आपकी योजना
guard-strip-report = आपकी रिपोर्ट
guard-strip-permission = मंज़ूरी
guard-strip-rescue = रेस्क्यू
guard-strip-undo = पूर्ववत करें
guard-strip-checking = जाँच

## X09 रीस्टार्ट के बाद जाँच, और स्थिति उपलब्ध नहीं

guard-checking-title = जाँच हो रही है कि क्या पूरा हुआ
guard-checking-body = PCTwin देख रहा है कि बंद होने से पहले क्या पूरा हो चुका था। पता चलते ही आपके विकल्प वापस आ जाएँगे।
guard-unavailable-title = स्थिति उपलब्ध नहीं है
guard-unavailable-body = PCTwin अभी नहीं बता सकता कि यह स्थानांतरण क्या कर रहा है। जब तक वह बता नहीं पाता, कुछ भी पूरा हुआ नहीं दिखाया जाएगा। थोड़ी देर में फिर से जाँच करें।

## 2.9 कुछ भी बदलने से पहले

guard-changes-title = स्थानांतरण क्या बदलेगा
guard-changes-body = यह वह सब है जो स्थानांतरण इस लैपटॉप पर बदलेगा, और हर चीज़ के लिए पूर्ववत करना क्या कर सकता है। अभी कुछ नहीं बदला है।
guard-changes-download = इंटरनेट से डाउनलोड
guard-changes-download-none = कुछ नहीं
guard-changes-permissions = सिस्टम की मंज़ूरी
guard-changes-one-prompt = आपका सिस्टम एक बार पूछता है
guard-changes-no-prompt = ज़रूरत नहीं
guard-changes-restore-point = Windows रिस्टोर पॉइंट
guard-changes-restore-point-first = पहले माँगा जाएगा
guard-changes-restore-point-none = इस्तेमाल नहीं होगा
guard-changes-install =
    { $count ->
        [one] इंस्टॉल करने के लिए: { $count } ऐप
       *[other] इंस्टॉल करने के लिए: { $count } ऐप्स
    }
guard-changes-merge =
    { $count ->
        [one] यहाँ जो है उसके साथ जोड़ा जाएगा: { $count } आइटम
       *[other] यहाँ जो है उसके साथ जोड़े जाएँगे: { $count } आइटम
    }
guard-changes-replace =
    { $count ->
        [one] बदलने के लिए, पुराना वर्शन रखते हुए: { $count } आइटम
       *[other] बदलने के लिए, पुराने वर्शन रखते हुए: { $count } आइटम
    }
guard-changes-leave-alone =
    { $count ->
        [one] जैसा है वैसा छोड़ा जाएगा: { $count } आइटम
       *[other] जैसे हैं वैसे छोड़े जाएँगे: { $count } आइटम
    }
guard-changes-space = सबसे व्यस्त समय में हर ड्राइव पर जगह
guard-changes-why-prompt = आपका सिस्टम क्यों पूछेगा
guard-changes-approve = ये बदलाव मंज़ूर करें
guard-undo-legend-can = PCTwin इसे पूर्ववत कर सकता है।
guard-undo-legend-keeps = PCTwin इसे रखता है। पूर्ववत करना इसे नहीं छूता।
guard-undo-legend-you = आपको करना होगा। इसे सिर्फ़ इसकी सेवा या बनाने वाला ही पूर्ववत कर सकता है।
guard-undo-can-reverse = PCTwin इसे पूर्ववत कर सकता है
guard-undo-can-remove-app = PCTwin ऐप हटा सकता है। यह आपकी बनाई फ़ाइलों को नहीं छूता।
guard-undo-must-keep = रखा गया: यह पहले से यहाँ था
guard-undo-needs-you-service = आपको करना होगा: इसे सिर्फ़ ऑनलाइन सेवा पूर्ववत कर सकती है
guard-undo-needs-you-maker = आपको करना होगा: साइन-इन और लाइसेंस बनाने वाले के होते हैं
guard-drive-needs = { $needed } चाहिए। { $free } खाली।
guard-drive-needs-free-unknown = { $needed } चाहिए। ड्राइव यह नहीं बताती कि कितनी जगह खाली है।
guard-drive-fits = आ जाएगा
guard-drive-short = { $short } कम है
guard-drive-unknown = पता नहीं
guard-permission-read-other-people = पुराने लैपटॉप पर दूसरे लोगों की फ़ाइलें पढ़ने के लिए
guard-permission-make-accounts = इस लैपटॉप पर उनके अकाउंट बनाने के लिए
guard-permission-shared-folders = ऐसे साझा फ़ोल्डरों में चीज़ें रखने के लिए जिन्हें आपका अकाउंट नहीं बदल सकता
guard-permission-install-for-everyone = इस लैपटॉप पर सबके लिए ऐप इंस्टॉल करने के लिए
guard-permission-restore-point = पहले Windows रिस्टोर पॉइंट बनाने के लिए

## X06 और H10 मंज़ूरी के बाद कुछ बदल गया

guard-changed-title =
    { $count ->
        [one] आपकी मंज़ूरी के बाद { $count } चीज़ बदल गई
       *[other] आपकी मंज़ूरी के बाद { $count } चीज़ें बदल गईं
    }
guard-changed-body = सिर्फ़ इन्हें दोबारा जाँचना है। बाकी सब मंज़ूर रहता है और जारी रहता है।
guard-changed-changed-on-old-laptop = यह पुराने लैपटॉप पर बदल गया।
guard-changed-app-updated = ऐप ने खुद को अपडेट कर लिया।
guard-changed-destination-changed = जहाँ यह जाना है, वह जगह बदल गई।
guard-changed-new-scope = यह नया है: कोई और व्यक्ति या लैपटॉप स्थानांतरण में जुड़ गया।
guard-changed-permission-ended = इसकी मंज़ूरी खत्म हो गई है।
guard-changed-leave-out = इन्हें छोड़ दें
guard-changed-approve = इन्हें फिर से मंज़ूर करें

## M31, M05 नए लैपटॉप पर जगह

guard-space-title = नए लैपटॉप पर जगह
guard-space-title-short = नए लैपटॉप पर जगह कम है
guard-space-body = PCTwin ने हर ड्राइव पर आने वाली हर चीज़ गिनी, पूर्ववत करने के लिए ज़रूरी जगह समेत। जगह में आने के लिए आपकी चुनी हुई कोई चीज़ नहीं छोड़ी जाती।
guard-space-proposal = जो नहीं आ पा रहा, उसके लिए PCTwin का सुझाव
guard-keep-off-leave-in-cloud = ऑनलाइन छोड़े गए, जहाँ वे अभी रखे हैं
guard-keep-off-offload-drive = आपकी दूसरी ड्राइव पर रखे गए
guard-keep-off-leave-on-old-laptop = पुराने लैपटॉप पर छोड़े गए। इनके स्थानांतरित होने तक उसे संभालकर रखें।
guard-offload-title = आपकी दूसरी ड्राइव
guard-offload-connected = जुड़ी है
guard-offload-not-connected = जुड़ी नहीं है
guard-offload-approved = आपने यह ड्राइव चुनी है। इस पर रखी फ़ाइलें नए लैपटॉप पर नहीं हैं: उन्हें खोलने के लिए इसे जोड़ें।
guard-offload-ask = PCTwin कुछ चीज़ें इस ड्राइव पर रख सकता है। इसके लिए इस ड्राइव की अलग मंज़ूरी चाहिए।
guard-offload-use = यह ड्राइव इस्तेमाल करें
guard-space-approve = यह योजना मंज़ूर करें
guard-safe-in = { $provider } में रखी हैं
guard-safe-in-cloud = ऑनलाइन रखी हैं
guard-safe-in-detail =
    { $count ->
        [one] { $count } फ़ाइल सिर्फ़ ऑनलाइन रखी है। नए लैपटॉप पर साइन इन करने पर यह वापस आ जाएगी।
       *[other] { $count } फ़ाइलें सिर्फ़ ऑनलाइन रखी हैं। नए लैपटॉप पर साइन इन करने पर वे वापस आ जाएँगी।
    }
guard-your-cloud = आपकी ऑनलाइन स्टोरेज

## M09 सिर्फ़ ऑनलाइन रखी फ़ाइलों की कॉपी रखना

guard-local-title = क्या आप सिर्फ़ ऑनलाइन रखी अपनी फ़ाइलों की कॉपी रखना चाहते हैं?
guard-local-body =
    { $count ->
        [one] { $provider } में { $count } फ़ाइल सिर्फ़ ऑनलाइन रखी है। आप उसकी एक कॉपी नए लैपटॉप पर भी रख सकते हैं।
       *[other] { $provider } में { $count } फ़ाइलें सिर्फ़ ऑनलाइन रखी हैं। आप उनकी एक कॉपी नए लैपटॉप पर भी रख सकते हैं।
    }
guard-local-download = पुराने लैपटॉप पर डाउनलोड करने के लिए
guard-local-old-space = पुराने लैपटॉप पर जगह
guard-local-how = PCTwin पुराने लैपटॉप पर उन्हें थोड़ी-थोड़ी करके डाउनलोड करता है, भेजता है, और फिर वहाँ उन्हें दोबारा सिर्फ़ ऑनलाइन कर देता है।
guard-local-paused-metered = रुका हुआ: इस कनेक्शन पर डेटा का पैसा लग सकता है। मुफ़्त कनेक्शन पर यह अपने आप जारी रहेगा, या आप अनुमति दें तो अभी।
guard-local-paused-no-space = रुका हुआ: पुराने लैपटॉप पर अगली कुछ फ़ाइलों के लिए जगह नहीं बची। बाकी { $provider } में सुरक्षित रहती हैं।
guard-local-done = हो गया। कॉपियाँ नए लैपटॉप पर हैं।
guard-local-keep = कॉपी रखें
guard-local-leave = उन्हें { $provider } में ही रहने दें
guard-local-allow-metered = इस कनेक्शन पर अनुमति दें

## M07 कौन कहाँ जाएगा

guard-owners-title = कौन कहाँ जाएगा
guard-owners-body = हर व्यक्ति की चीज़ें इस लैपटॉप पर उसके अपने अकाउंट में जाती हैं। सिर्फ़ नाम मिलने से यह कभी तय नहीं होता।
guard-owners-from = { $laptop } से
guard-owners-to-account = इस लैपटॉप के अकाउंट { $account } में
guard-owners-new-account = इस लैपटॉप के नए अकाउंट में
guard-owners-own-folder = उनके अपने “{ $laptop } से” फ़ोल्डर, क्योंकि नाम टकरा रहे हैं
guard-owners-usual-folders = उनके आम फ़ोल्डर
guard-owners-approve = मंज़ूर करें

## X12 जो पूरा नहीं हुआ

guard-report-title-partial = कुछ चीज़ें स्थानांतरित नहीं हुईं
guard-report-title-done = इस हिस्से की सारी चीज़ें स्थानांतरित हो गईं
guard-report-body-partial = नीचे दी हर चीज़ अभी पुराने लैपटॉप पर है। तैयार हों तो हर एक को फिर से आज़माएँ। जो पहले ही स्थानांतरित हो चुका है, वह वहीं रहेगा।
guard-report-body-done = फिर से आज़माने के लिए कुछ नहीं बचा।
guard-line-app = ऐप { $name }
guard-line-items =
    { $count ->
        [one] { $count } आइटम
       *[other] { $count } आइटम
    }
guard-line-settings =
    { $count ->
        [one] { $count } सेटिंग
       *[other] { $count } सेटिंग्स
    }
guard-line-why-failed = { $laptop } से: यह पूरा नहीं हुआ।
guard-line-why-laptop-not-connected = { $laptop } से: वह लैपटॉप जुड़ा नहीं है। उसे उसी Wi-Fi से जोड़ें और चालू रखें।
guard-line-why-waits-for-another = { $laptop } से: यह एक दूसरे चरण का इंतज़ार कर रहा है जो पूरा नहीं हुआ। पहले उसे ठीक करें।
guard-line-why-not-started = { $laptop } से: स्थानांतरण शुरू होने से पहले ही रुक गया।

## M25, H10 सिस्टम की मंज़ूरी, और रिस्टोर पॉइंट

guard-permission-title-explain = PCTwin को आपके सिस्टम से एक मंज़ूरी चाहिए
guard-permission-title-waiting-for-system = अपने सिस्टम के सवाल का जवाब दें
guard-permission-title-denied = आपने मंज़ूरी नहीं दी
guard-permission-title-closed-without-answer = सवाल बिना जवाब के बंद कर दिया गया
guard-permission-title-given = इस स्थानांतरण के लिए मंज़ूर
guard-permission-title-ended-by-sleep = लैपटॉप के स्लीप में जाने पर मंज़ूरी खत्म हो गई
guard-permission-title-ended-other-session = किसी और के साइन इन करने पर मंज़ूरी खत्म हो गई
guard-permission-body-explain = आपका सिस्टम पूरे स्थानांतरण के लिए एक ही बार पूछता है। इसे सिर्फ़ नीचे दिए चरण इस्तेमाल करते हैं, और यह स्थानांतरण के साथ खत्म हो जाती है।
guard-permission-body-waiting-for-system = आपके सिस्टम की अपनी विंडो खुली है। अगर वह माँगे तो पासवर्ड वहीं डालें।
guard-permission-body-denied = सिर्फ़ वही चरण छोड़े जाएँगे जिन्हें इसकी ज़रूरत थी। बाकी सब आगे बढ़ता है।
guard-permission-body-closed-without-answer = विंडो बंद करना मंज़ूरी नहीं माना जाता, इसलिए जिसे इसकी ज़रूरत है वह कुछ नहीं किया गया। फिर से माँगें, या इसके बिना आगे बढ़ें।
guard-permission-body-given = PCTwin इसे सिर्फ़ नीचे दिए चरणों के लिए इस्तेमाल करता है, और यह इस स्थानांतरण के साथ खत्म हो जाती है।
guard-permission-body-ended-by-sleep = इसके बाद जिसे इसकी ज़रूरत है वह कुछ नहीं किया गया। आगे बढ़ने के लिए फिर से माँगें।
guard-permission-body-ended-other-session = इसके बाद जिसे इसकी ज़रूरत है वह कुछ नहीं किया गया। अपने अकाउंट पर लौटने पर फिर से माँगें।
guard-permission-never-password = PCTwin कभी आपका पासवर्ड नहीं माँगता और कभी उसे देखता नहीं। सिर्फ़ आपका सिस्टम देखता है।
guard-permission-not-now = इसके बिना आगे बढ़ें
guard-permission-ask = अपने सिस्टम पर जाएँ
guard-permission-ask-again = फिर से माँगें
guard-restore-title = Windows रिस्टोर पॉइंट
guard-restore-made = { $time } पर बनाया गया।
guard-restore-earlier-exists = Windows के पास { $time } का एक पहले से है, इसलिए उसने दूसरा नहीं बनाया।
guard-restore-not-chosen = नहीं बनाया गया: आपने न बनाना चुना।
guard-restore-not-made-not-windows = नहीं बनाया गया: रिस्टोर पॉइंट सिर्फ़ Windows पर होते हैं।
guard-restore-not-made-protection-off = नहीं बनाया गया: इस ड्राइव के लिए System Protection बंद है। आप इसे System Properties में चालू कर सकते हैं।
guard-restore-not-made-needs-admin = नहीं बनाया गया: इसके लिए आपके सिस्टम की मंज़ूरी चाहिए।
guard-restore-not-made-unconfirmed = नहीं बनाया गया: Windows ने हाँ कहा, पर कोई रिस्टोर पॉइंट नहीं मिल रहा।
guard-restore-not-made-failed = नहीं बनाया गया: Windows एक नहीं बना सका।

## T01 से T04 ऐसे लैपटॉप से रेस्क्यू जो चालू नहीं होता

guard-rescue-title-intro = ऐसे लैपटॉप से फ़ाइलें कॉपी करें जो चालू नहीं होता
guard-rescue-title-pick-drive = बचाने के लिए ड्राइव चुनें
guard-rescue-title-locked = यह ड्राइव लॉक है
guard-rescue-title-no-system = यह किसी लैपटॉप की ड्राइव नहीं है
guard-rescue-title-salvage = जो पढ़ा जा सका, वह यह है
guard-rescue-body-intro = पुराने लैपटॉप से ड्राइव निकालें और ड्राइव एडेप्टर या एन्क्लोज़र से उसे इस लैपटॉप से जोड़ें।
guard-rescue-body-pick-drive = पुराने लैपटॉप की ड्राइव चुनें। जिस ड्राइव पर आप कॉपी करते हैं, वह वही नहीं हो सकती जिसे आप बचा रहे हैं।
guard-rescue-body-locked = पहले इसे अपने सिस्टम में अनलॉक करें। फिर दोबारा जाँचें।
guard-rescue-body-no-system = इस ड्राइव पर लैपटॉप का सिस्टम और होम फ़ोल्डर नहीं हैं। अगर यह आम USB ड्राइव है, तो इसे नए लैपटॉप पर इस्तेमाल करें।
guard-rescue-body-salvage = सूची देखें, फिर जो पढ़ा जा सका उसे इस्तेमाल करें।
guard-rescue-can = रेस्क्यू क्या कर सकता है
guard-rescue-can-body = ऐसी ड्राइव से, जो अब भी चलती है, आपकी फ़ाइलें और ड्राइव पर मिले लोग और सेटिंग्स पढ़ना।
guard-rescue-cannot = वह क्या नहीं कर सकता
guard-rescue-cannot-body = ड्राइव की मरम्मत करना, खराब हिस्से पढ़ना, या ऐसी ड्राइव खोलना जिसकी चाबी आपके पास नहीं है।
guard-rescue-read-only-title = ड्राइव सिर्फ़ पढ़ी जाती है
guard-rescue-read-only-promise = PCTwin इसे सिर्फ़ पढ़ता है। इस पर कुछ भी बदला, ठीक या फ़ॉर्मैट नहीं किया जाता।
guard-rescue-system-read-only = आपके सिस्टम ने इसे सिर्फ़ पढ़ने के लिए खोला, इसलिए इस पर कुछ नहीं बदल सकता।
guard-rescue-pctwin-reads-only = PCTwin ने इसे सिर्फ़ पढ़ा। आपके सिस्टम ने इसे बदलाव के लिए खोल रखा था, इसलिए दूसरे ऐप अब भी इसे बदल सकते थे।
guard-rescue-refused-target = वह ड्राइव वही है जिस पर चीज़ें रखी जाती हैं, इसलिए उसे बचाया भी नहीं जा सकता। दूसरी ड्राइव चुनें।
guard-rescue-drive-detail = { $size }, { $fs }
guard-rescue-drive-is-target = स्थानांतरण की चीज़ें इस ड्राइव पर जाती हैं।
guard-rescue-drive-cannot = बचाई नहीं जा सकती
guard-rescue-choose = यह ड्राइव बचाएँ
guard-rescue-bit-locker-step-1 = File Explorer में ड्राइव खोलें।
guard-rescue-bit-locker-step-2 = Windows के माँगने पर उसकी रिकवरी चाबी डालें। वह अक्सर मालिक के Microsoft अकाउंट में सेव होती है, aka.ms/myrecoverykey पर।
guard-rescue-bit-locker-step-3 = यहाँ लौटें और फिर से जाँच करें चुनें।
guard-rescue-file-vault-step-1 = इसे Mac पर मालिक के लॉगिन पासवर्ड या रिकवरी चाबी से अनलॉक करें।
guard-rescue-file-vault-step-2 = यहाँ लौटें और फिर से जाँच करें चुनें।
guard-rescue-apple-chip-step-1 = यह Mac अपनी ड्राइव को सिर्फ़ अपने लिए लॉक रखता है, इसलिए ड्राइव निकालने से काम नहीं चलता।
guard-rescue-apple-chip-step-2 = अगर Mac अब भी Recovery में चालू होता है, तो केबल के साथ Share Disk इस्तेमाल करें। अगर वह बिल्कुल चालू नहीं होता, तो सिर्फ़ Apple का रिपेयर सेंटर मदद कर सकता है।
guard-rescue-never-key = PCTwin कभी चाबी नहीं माँगता और कभी उसे रखता नहीं।
guard-rescue-readable =
    { $count ->
        [one] { $n } आइटम पढ़ा जा सकता है।
       *[other] { $n } आइटम पढ़े जा सकते हैं।
    }
guard-rescue-unreadable =
    { $count ->
        [one] { $count } आइटम पढ़ा नहीं जा सका
       *[other] { $count } आइटम पढ़े नहीं जा सके
    }
guard-rescue-look = ड्राइव ढूँढें
guard-rescue-check-again = फिर से जाँच करें
guard-rescue-use = जो पढ़ा जा सका उसे इस्तेमाल करें

## R05, R06 पूर्ववत करना

guard-undo-preview-title = पूर्ववत करना क्या करेगा
guard-undo-preview-body = PCTwin ने अभी हर बदलाव को देखा। स्थानांतरण के बाद आपने जो बदला, वह रखा जाता है। जब तक आप पूर्ववत करें नहीं चुनते, कुछ नहीं होता।
guard-undo-preview-which = यह { DATETIME($when, year: "numeric", month: "long", day: "numeric", hour: "numeric", minute: "2-digit") } को पूरे हुए स्थानांतरण को पूर्ववत करता है। आपके दूसरे स्थानांतरण नहीं बदलते।
guard-undo-preview-which-earlier = यह PCTwin के पुराने वर्शन से किए गए स्थानांतरण को पूर्ववत करता है। आपके दूसरे स्थानांतरण नहीं बदलते।
guard-undo-preview-admin = ऐप हटाने के लिए यह लैपटॉप किसी व्यवस्थापक से मंज़ूरी मांगेगा।
guard-undo-plan-put-back = जैसा था वैसा वापस रखा गया
guard-undo-plan-remove-app = PCTwin ऐप हटाता है। PCTwin आपकी बनाई फ़ाइलों को नहीं छूता।
guard-undo-plan-keep-changed = रखा गया: आपने इसे बदला है
guard-undo-plan-elsewhere = आपको करना होगा: कहीं और किया गया
guard-undo-plan-leave-alone = छोड़ दिया गया: यह पहले से यहाँ था
guard-undo-keep = सब कुछ जैसा है वैसा रखें
guard-undo-run = पूर्ववत करें
guard-undo-running-title = बदलाव पूर्ववत किए जा रहे हैं
guard-undo-done-title = पूर्ववत करने पर क्या हुआ
guard-undo-result-restored = वापस लाया गया
guard-undo-result-removed = हटाया गया
guard-undo-result-as-before = पहले जैसा ही था
guard-undo-result-kept-changed = रखा गया क्योंकि आपने इसे बदला है
guard-undo-result-kept-other = रखा गया, और जानकारी देखें
guard-undo-result-kept-at = { $name } नाम से रखा गया
guard-undo-result-done-elsewhere = कहीं और किया गया: इसे सिर्फ़ इसकी सेवा या बनाने वाला ही पूर्ववत कर सकता है
guard-undo-result-not-done-yet = अभी नहीं हुआ। पूर्ववत करना अगली बार फिर कोशिश करेगा।

## The full move: apps, settings and app data (28 October 2026)

fullmove-plan-title = ऐप, सेटिंग्स और ऐप डेटा
fullmove-plan-body = PCTwin ने आपके पुराने लैपटॉप के ऐप, सेटिंग्स और ऐप डेटा देखे। आपकी फ़ाइलों के बाद यह इस लैपटॉप पर यही करेगा। जब तक आप मूव शुरू नहीं करते, कुछ नहीं बदलता।
fullmove-group-install = ऐप जिन्हें PCTwin इंस्टॉल करेगा: { $count }
fullmove-group-settings = ऐप सेटिंग्स जिन्हें PCTwin कॉपी करेगा: { $count }
fullmove-group-needs = काम जो आप खुद करेंगे: { $count }
fullmove-group-here = ऐप जो इस लैपटॉप पर पहले से हैं: { $count }
fullmove-settings-count = सेटिंग्स: { $count }।
fullmove-settings-merge = ये जोड़ी जाती हैं। इस लैपटॉप पर कुछ भी बदला नहीं जाता।
fullmove-settings-replace = इस लैपटॉप की कुछ सेटिंग्स बदली जाती हैं। पूर्ववत करना उन्हें वापस ला सकता है।
fullmove-app-install = PCTwin इसे इसके बनाने वाले से इंस्टॉल करता है और जाँचता है।
fullmove-app-here = यह इस लैपटॉप पर पहले से है, इसलिए PCTwin इसे वैसा ही छोड़ता है।
fullmove-step-copy = कॉपी किया गया, पूर्ववत करने के साथ।
fullmove-step-export-first = अपने पुराने लैपटॉप पर ऐप का अपना एक्सपोर्ट इस्तेमाल करें। फिर इसे इस लैपटॉप के ऐप में इंपोर्ट करें।
fullmove-step-sign-in = इस लैपटॉप पर इसमें फिर से साइन इन करें।
fullmove-step-relink = इसे इस लैपटॉप पर फिर से सेट करें, या ऐप से फिर से जोड़ें।
fullmove-step-not-yet = PCTwin अभी इस ऐप का डेटा नहीं ले जाता। यह आपके पुराने लैपटॉप पर सुरक्षित है।
fullmove-step-look-at-it = PCTwin पता नहीं लगा सका कि यह क्या है। इसे अपने पुराने लैपटॉप पर देखें।
fullmove-group-update =
    { $count ->
        [one] इस लैपटॉप पर एक पुराना संस्करण है
       *[other] इस लैपटॉप पर पुराने संस्करण हैं
    }
fullmove-update-it = { $name } अपडेट करें
fullmove-update-hint = PCTwin नया संस्करण इंस्टॉल करेगा। आपकी सेटिंग वैसी ही रहेंगी।
fullmove-special-not-yet = ऐप डेटा, जैसे ईमेल, ब्राउज़र और गेम
fullmove-left-by-person = ऐप, सेटिंग और ऐप डेटा: आपकी पसंद के अनुसार पुराने लैपटॉप पर ही रहे
fullmove-special-finish = PCTwin ने इसे नहीं ले जाया। यह आपके पुराने लैपटॉप पर सुरक्षित है।
fullmove-more = और { $count } और।
fullmove-not-moved-title = सेटिंग्स जो PCTwin पुराने लैपटॉप पर छोड़ता है
fullmove-not-moved-private = निजी और सुरक्षा सेटिंग्स, और साइन-इन, जो कभी कॉपी नहीं होते: { $count }
fullmove-not-moved-could-run = सेटिंग्स जो कुछ शुरू करतीं या खोलतीं, सुरक्षा के लिए रोकी गईं: { $count }
fullmove-not-moved-old-laptop-only = सेटिंग्स जो सिर्फ़ पुराने लैपटॉप पर काम की हैं: { $count }
fullmove-not-moved-system-own = सिस्टम की अपनी सेटिंग्स जिन्हें PCTwin अभी नहीं ले जाता: { $count }
fullmove-not-moved-not-read = मिली पर पढ़ी नहीं गई सेटिंग्स: { $count }
fullmove-special-not-moved = छोड़ी गई ऐप फ़ाइलें, जैसे कैश: { $count }
fullmove-prompt = यह लैपटॉप एक बार एडमिनिस्ट्रेटर की अनुमति माँगेगा।
fullmove-prompt-restore = यह लैपटॉप एक बार एडमिनिस्ट्रेटर की अनुमति माँगेगा, और पहले बदलाव से पहले रिस्टोर पॉइंट बनाएगा।
fullmove-rebuilding-title = आपके ऐप और सेटिंग्स सेट हो रहे हैं
fullmove-rebuilding-body = आपकी फ़ाइलें अपनी जगह पर हैं। PCTwin अब बाकी सेट कर रहा है। इस लैपटॉप को चालू रखें।
fullmove-rebuilding-permission = इस लैपटॉप से अनुमति माँगी जा रही है
fullmove-rebuilding-apps = ऐप: { $total } में से { $done }
fullmove-rebuilding-settings = सेटिंग्स: { $total } में से { $done }
fullmove-outcome-title = ऐप, सेटिंग्स और ऐप डेटा
fullmove-outcome-undo = ऐप सेटिंग्स पूर्ववत करें
fullmove-outcome-stopped = PCTwin ने सेटिंग्स बदलना बीच में रोक दिया, क्योंकि वह अपना अगला बदलाव लिख नहीं सका। उसके बाद कुछ नहीं बदला।
fullmove-restore-made = बदलावों से पहले एक रिस्टोर पॉइंट बनाया गया।
fullmove-restore-earlier-exists = नया रिस्टोर पॉइंट नहीं बना, क्योंकि इस लैपटॉप पर आज का एक पहले से था।
fullmove-restore-not-made = कोई रिस्टोर पॉइंट नहीं बना। PCTwin का अपना पूर्ववत करना अब भी काम करता है।
fullmove-restore-not-windows = इस लैपटॉप पर रिस्टोर पॉइंट नहीं होते। PCTwin का अपना पूर्ववत करना अब भी काम करता है।
guard-undo-remove-yourself = चाहें तो इसे खुद हटाएँ
guard-undo-plan-remove-yourself = आप इसे अपने लैपटॉप की सेटिंग्स में हटा सकते हैं
guard-undo-result-remove-yourself = आपके हटाने के लिए छोड़ा गया, लैपटॉप की सेटिंग्स में
guard-undo-plan-keep-other-version = रखा गया: स्थानांतरण के बाद यह बदल गया
guard-undo-plan-already-gone = पहले ही हटाया जा चुका है
guard-undo-result-kept-other-version = रखा गया, क्योंकि स्थानांतरण के बाद यह बदल गया
guard-undo-result-removed-restart = हटाया गया। पूरा करने के लिए अपना लैपटॉप फिर से चालू करें।
guard-undo-result-not-removed = नहीं हटाया गया। आप इसे अपने लैपटॉप की सेटिंग्स में हटा सकते हैं।
guard-undo-close = हो गया
move-wait-appdata-title = आपके ऐप डेटा को देखा जा रहा है
move-wait-appdata-body = आपकी फ़ाइलें मिल गई हैं। PCTwin अभी आपके ऐप्स का रखा डेटा देख रहा है, जैसे ईमेल और ब्राउज़र। इसमें ज़्यादा से ज़्यादा दस मिनट लगते हैं। इस लैपटॉप पर कुछ नहीं बदलता।
fullmove-special-not-checked = ऐप डेटा जिसे PCTwin पूरी तरह जाँच नहीं सका
fullmove-group-appdata = ऐप डेटा जिसे PCTwin कॉपी करेगा: { $count }
fullmove-group-secrets = गुप्त फ़ाइलें जिन्हें आप लाना चुन सकते हैं: { $count }
fullmove-secrets-body = इनमें पासवर्ड या कुंजियाँ होती हैं। PCTwin कोई भी तभी लाता है जब आप खुद उस पर टिक करें। बिना टिक के, वह आपके पुराने लैपटॉप पर रहती है।
fullmove-secret-bring = { $name } लाएँ
fullmove-secret-hint = इस लैपटॉप पर इसे सिर्फ़ आप खोल सकेंगे। पूर्ववत करना इसे फिर से हटा सकता है।
fullmove-appdata-title = आपके ऐप डेटा की कॉपी हो रही है
fullmove-appdata-progress = ऐप डेटा की कॉपी: { $total } में से { $done } ऐप
fullmove-appdata-waiting = पुराने लैपटॉप का इंतज़ार: वहाँ { $apps } बंद करें।
fullmove-close-title = इनका डेटा लाने के लिए ये ऐप बंद करें
fullmove-close-body = ये ऐप इस लैपटॉप पर अभी खुले हैं, इसलिए PCTwin इनका डेटा सुरक्षित रूप से कॉपी नहीं कर सकता। इन्हें बंद करें, फिर फिर से कोशिश करें चुनें। इनके बिना आगे बढ़ने पर इनका डेटा यहीं रहता है।
fullmove-close-again = फिर से कोशिश करें
fullmove-close-without = इनके बिना आगे बढ़ें
fullmove-extras-refused = पुराने लैपटॉप की सेटिंग्स और ऐप डेटा यहाँ पढ़े नहीं जा सके, इसलिए कुछ नहीं ले जाया गया। वे आपके पुराने लैपटॉप पर अब भी हैं।
fullmove-app-from = PCTwin winget पैकेज { $id } उसके बनाने वाले से इंस्टॉल करता है, यह जाँचने के बाद कि यह यही ऐप है और बनाने वाला वही है।
fullmove-here-older = इस लैपटॉप पर एक पुराना संस्करण ({ $version }) है। PCTwin उसे वैसा ही छोड़ता है।
fullmove-step-sites = वेबसाइटों पर सहेजे गए साइन-इन पुराने लैपटॉप से बंधे रहते हैं। इस लैपटॉप पर उन वेबसाइटों में फिर से साइन इन करें।
fullmove-app-rosetta = यह पुराने Intel Mac के लिए बना है। इस Mac पर यह Rosetta के ज़रिए चलता है, जिसे Apple बंद कर रहा है, इसलिए macOS के किसी बाद के अपडेट के बाद यह काम करना बंद कर सकता है। इसके निर्माता से नया संस्करण देखें।
fullmove-links = आपकी फ़ाइलों में दूसरी जगहों के शॉर्टकट, जिन्हें PCTwin न खोलता है न ले जाता है: { $count }
fullmove-note-sign-in = इसे खोलें और अपने खाते में साइन इन करें।
fullmove-note-licence-key = इसे खोलें और अपनी लाइसेंस कुंजी डालें, या उस खाते में साइन इन करें जिसमें आपका लाइसेंस है।
fullmove-note-registration = आपके पुराने लैपटॉप पर यह { $who } के नाम पंजीकृत था।
fullmove-note-product-id = आपके पुराने लैपटॉप पर इसकी उत्पाद ID { $id } थी।
fullmove-note-extras = इसके साथ दूसरे प्रोग्राम भी आए: { $apps }। अगर आप उन्हें नहीं चाहते, तो लैपटॉप की सेटिंग में उन्हें हटा दें।
fullmove-note-rosetta = यह Intel Mac के लिए बना है, इसलिए इसे Rosetta चाहिए। ऐप खोलने पर अगर यह Mac Rosetta इंस्टॉल करने को कहे, तो इंस्टॉल चुनें।
fullmove-data-lost = यह डेटा आने से पहले कनेक्शन टूट गया। यह अब भी आपके पुराने लैपटॉप पर सुरक्षित है। इसे लाने के लिए नया स्थानांतरण शुरू करें।
fullmove-data-did-not-arrive = यह डेटा नहीं आया। यह अब भी आपके पुराने लैपटॉप पर सुरक्षित है। इसे लाने के लिए नया स्थानांतरण शुरू करें।
fullmove-data-app-open = आपके पुराने लैपटॉप पर ऐप खुला था, इसलिए उसका डेटा कॉपी नहीं हुआ। वहाँ ऐप बंद करें, फिर इसे लाने के लिए नया स्थानांतरण शुरू करें।
fullmove-data-changing = PCTwin के पढ़ते समय यह डेटा आपके पुराने लैपटॉप पर बदलता रहा। वहाँ ऐप बंद करें, फिर इसे लाने के लिए नया स्थानांतरण शुरू करें।
fullmove-data-unreadable = PCTwin आपके पुराने लैपटॉप पर यह डेटा पढ़ नहीं सका। यह अब भी वहीं है।
fullmove-data-not-read = यह डेटा पढ़े जाने से पहले स्थानांतरण रुक गया। यह अब भी आपके पुराने लैपटॉप पर सुरक्षित है।
fullmove-data-not-whole = यह सारा डेटा नहीं आया, इसलिए PCTwin ने इसका कुछ भी अपनी जगह नहीं रखा। यह अब भी आपके पुराने लैपटॉप पर सुरक्षित है।
fullmove-data-not-landed = यह डेटा आ गया, पर PCTwin इसे इस लैपटॉप पर अपनी जगह नहीं रख सका। यह अब भी आपके पुराने लैपटॉप पर सुरक्षित है।
fullmove-data-beside = इस लैपटॉप पर ऐप का अपना फ़ोल्डर पहले से था, इसलिए PCTwin ने पुराने लैपटॉप की कॉपी उसके बगल में, { $folder } में रखी। ऐप अब भी अपना ही फ़ोल्डर इस्तेमाल करता है। पुरानी कॉपी इस्तेमाल करने के लिए ऐप बंद करें और दोनों फ़ोल्डर आपस में बदल दें।
fullmove-data-placed = डेटा { $folder } में है, पर ऐप को इसे इस्तेमाल करने से पहले एक कदम चाहिए। ऐप खोलें और इसे जाँचें।
fullmove-outcome-here-title = इस लैपटॉप पर पहले से मौजूद ऐप: { $count }
fullmove-outcome-here = इस लैपटॉप पर पहले से है, वैसा ही छोड़ा गया।
fullmove-outcome-here-older = इस लैपटॉप पर एक पुराना संस्करण ({ $version }) है। PCTwin ने उसे वैसा ही छोड़ा। नया संस्करण चाहिए तो उसे उसके निर्माता से अपडेट करें।
fullmove-outcome-here-badge = वैसा ही छोड़ा गया
fullmove-outcome-fresh = PCTwin के इंस्टॉल किए ऐप नए सिरे से शुरू होते हैं। हर ऐप खोलें और माँगे तो साइन इन करें।
progress-may-sleep = स्थानांतरण के दौरान यह लैपटॉप स्लीप में जा सकता है। इसे चार्ज पर लगा और खुला रखें।

## The old laptop chooses what moves (decided 28 October 2026)
choose-title = क्या ले जाना है?
choose-body = यहीं चुनें, उस लैपटॉप पर जहाँ आपकी चीज़ें हैं। फिर आपका नया लैपटॉप दिखाएगा कि क्या आ रहा है और आपसे उसे स्वीकार करने को कहेगा।
choose-mode = क्या ले जाएँ
choose-everything = सब कुछ
choose-except = सब कुछ, इनके सिवा
choose-only = सिर्फ़ जो मैं चुनूँ
choose-except-hint = जो छोड़ना है उससे निशान हटाएँ।
choose-open = { $name } खोलें
choose-close = { $name } बंद करें
choose-total =
    { $files ->
        [one] 1 फ़ाइल
       *[other] { $files } फ़ाइलें
    } · चुना गया: { $size }
choose-only-hint = जो ले जाना है उस पर निशान लगाएँ।
choose-apps = ऐप, सेटिंग और ऐप डेटा
choose-apps-count =
    { $count ->
        [one] 1 ऐप, उसकी सेटिंग के साथ
       *[other] { $count } ऐप, उनकी सेटिंग के साथ
    }
choose-group-size =
    { $files ->
        [one] 1 फ़ाइल
       *[other] { $files } फ़ाइलें
    } · { $size }
choose-left-title = ये वैसे भी यहीं रहेंगे
choose-left-badge = यहीं रहेगा
choose-left-caches = कैश और अस्थायी फ़ाइलें
choose-left-chosen-by-user = जो आपने छोड़ना चुना
choose-left-work-managed = आपके दफ़्तर या स्कूल की फ़ाइलें
choose-left-device-bound = इस लैपटॉप से जुड़ी चीज़ें, जैसे पासकी
choose-left-unreadable = वे फ़ाइलें जिन्हें PCTwin पढ़ नहीं सका
choose-left-cloud-only = सिर्फ़ ऑनलाइन रखी फ़ाइलें
choose-left-system = Windows के अपने हिस्से
choose-left-too-many-files = एक बार में ले जाने के लिए बहुत बड़े फ़ोल्डर
choose-left-rebuildable = जो चीज़ें आपका नया लैपटॉप फिर से बना सकता है, जैसे डाउनलोड किए पैकेज
choose-nothing = ले जाने के लिए कम से कम एक चीज़ चुनें।
choose-continue = आगे बढ़ें

## The new laptop accepts what the old laptop chose (decided 28 October 2026)
accept-title = { $from } से स्थानांतरण स्वीकार करें?
accept-body = यह आपके पुराने लैपटॉप पर चुना गया। स्वीकार करें, और PCTwin इसे यहाँ ले आएगा।
accept-files =
    { $count ->
        [one] 1 फ़ाइल
       *[other] { $count } फ़ाइलें
    }
accept-apps =
    { $count ->
        [one] 1 ऐप इंस्टॉल होगा
       *[other] { $count } ऐप इंस्टॉल होंगे
    }
accept-settings = सेटिंग और ऐप डेटा
accept-fits = यह इस लैपटॉप पर आ जाएगा।
accept-needs = पहले किसी चीज़ पर आपका ध्यान चाहिए। विवरण देखें।
accept-not-now = अभी नहीं
accept-details = विवरण देखें
accept-accept = स्वीकार करें

# 2.5 special categories
special-category-email = ईमेल
special-category-browser = ब्राउज़र
special-category-game = गेम
special-category-ai-tool = AI टूल और कोड एडिटर
special-category-secret = कुंजियाँ और पासवर्ड
special-category-other = ऐप का बाकी डेटा
special-note-search-rebuilds = आपका मेल ऐप पहली बार खुलने पर अपनी खोज फिर से बनाता है। कुछ समय तक खोज धीमी हो सकती है।
special-note-server-resync = साइन इन करने पर आपका मेल मेल सर्वर से वापस आ जाता है। PCTwin पुरानी कॉपी सिर्फ़ इसलिए रखता है कि शायद कुछ सर्वर तक कभी न पहुँचा हो।
special-note-own-sync = सबसे आसान तरीका: नए लैपटॉप पर इस ब्राउज़र का अपना सिंक चालू करें। इससे आपके बुकमार्क, इतिहास, खुले टैब और ऐड-ऑन आ जाते हैं।
special-note-passwords-guided = सेव किए गए पासवर्ड सिर्फ़ ब्राउज़र के अपने एक्सपोर्ट और इंपोर्ट से, या उसके सिंक से जाते हैं। एक्सपोर्ट की गई फ़ाइल सुरक्षित नहीं होती, इसलिए PCTwin उसे निजी रखता है और इंपोर्ट के बाद हटा देता है।
special-note-passkeys-guided = किसी खाते में सेव की गई पासकी उसमें साइन इन करने पर वापस आ जाती हैं। जो पासकी सिर्फ़ इस लैपटॉप पर हैं, उन्हें हर वेबसाइट पर फिर से बनाना होगा। पुराना लैपटॉप मिटाने से पहले यह कर लें।
special-note-cloud-saves = गेम का लॉन्चर आपकी सेव फ़ाइलें ऑनलाइन भी रख सकता है। अगर वह पूछे कि कौन-सी सेव रखनी हैं, तो पहले तारीखें देख लें।
special-note-models-checked = AI मॉडल पहुँचने पर एक-एक फ़ाइल करके जाँचे जाते हैं। PCTwin उन्हें कभी नहीं चलाता।
special-note-tool-settings-review = ये सेटिंग्स प्रोग्राम शुरू कर सकती हैं या इंटरनेट से जुड़ सकती हैं। नए लैपटॉप पर इस्तेमाल करने से पहले इन्हें देख लें।
special-note-secret-tick-each = यह तभी जाएगा जब आप इस पर टिक करेंगे। यह किसी रिपोर्ट में कभी नहीं दिखता।
forgotten-title = चीज़ें जो लोग अक्सर भूल जाते हैं
forgotten-kind-fonts = आपके अपने फ़ॉन्ट
forgotten-kind-mail-signatures = मेल सिग्नेचर
forgotten-kind-dictionaries = आपके अपने शब्दकोश शब्द
forgotten-kind-office-templates = Office टेम्पलेट
forgotten-kind-mail-archives = इस लैपटॉप पर रखा मेल
forgotten-kind-game-saves = गेम की सेव फ़ाइलें
forgotten-kind-game-library = गेम लॉन्चर से इंस्टॉल किए गए गेम
forgotten-moving = यह आपकी बाकी चीज़ों के साथ जाएगा।
forgotten-not-moving = यह इस मूव में नहीं है। आप इसे खुद कॉपी कर सकते हैं।
forgotten-step-install = मूव के बाद, नए लैपटॉप पर हर फ़ॉन्ट खोलें और इंस्टॉल चुनें।
forgotten-step-locate-verify = नए लैपटॉप पर गेम लॉन्चर इंस्टॉल करें और साइन इन करें। वह आपके गेम फिर से इंस्टॉल कर सकता है। अगर आप यह फ़ोल्डर कॉपी करते हैं, तो लॉन्चर का इंस्टॉल किए गए गेम ढूँढने वाला विकल्प इस्तेमाल करें और उसे फ़ाइलें जाँचने दें।
forgotten-not-whole = PCTwin ने यहाँ गिनना रोक दिया, इसलिए और भी हो सकते हैं।
forgotten-printers = फिर से जोड़ने वाले प्रिंटर
forgotten-printer-step = इस प्रिंटर को नए लैपटॉप पर फिर से जोड़ें।
forgotten-printer-network = किसी दूसरे कंप्यूटर से शेयर किया गया
forgotten-not-checked = PCTwin यहाँ नहीं देख सका।
forgotten-files =
    { $count ->
        [one] 1 फ़ाइल
       *[other] { $count } फ़ाइलें
    }
forgotten-online-only =
    { $count ->
        [one] 1 फ़ाइल सिर्फ़ ऑनलाइन है, डाउनलोड नहीं की गई
       *[other] { $count } फ़ाइलें सिर्फ़ ऑनलाइन हैं, डाउनलोड नहीं की गईं
    }

# 2.7 finishing a move
doctor-not-connected = यह लैपटॉप किसी नेटवर्क से नहीं जुड़ा है।
doctor-not-connected-fix = Wi-Fi से जुड़ें या नेटवर्क केबल लगाएँ, फिर दोबारा कोशिश करें।
doctor-mac-local-network-denied = हो सकता है यह Mac PCTwin को आपके नेटवर्क के डिवाइसों से बात न करने दे रहा हो।
doctor-mac-local-network-denied-fix = System Settings खोलें, फिर Privacy & Security, फिर Local Network, और PCTwin चालू करें।
doctor-discovery-blocked = इस लैपटॉप पर कोई चीज़, जैसे फ़ायरवॉल या सुरक्षा ऐप, लैपटॉपों को एक-दूसरे को ढूँढने से रोक रही हो सकती है।
doctor-discovery-blocked-fix = अपने फ़ायरवॉल या सुरक्षा ऐप में PCTwin को अनुमति दें, या मूव के दौरान उसे रोक दें।
doctor-windows-public-network = Windows नेटवर्क { $name } को सार्वजनिक मानता है, जिससे दूसरे डिवाइस रुक जाते हैं।
doctor-windows-public-network-fix = Settings खोलें, फिर Network & internet, { $name } चुनें, और उसे निजी नेटवर्क पर सेट करें।
doctor-vpn-on = एक VPN चालू है ({ $name })। यह आपके घर के नेटवर्क को छिपा सकता है।
doctor-vpn-on-fix = मूव के दौरान VPN बंद करें, फिर दोबारा कोशिश करें।
doctor-old-laptop-not-pairing = हो सकता है पुराना लैपटॉप अपना कोड न दिखा रहा हो।
doctor-old-laptop-not-pairing-fix = पुराने लैपटॉप पर PCTwin खोलें और चुनें कि यह आपका पुराना लैपटॉप है।
doctor-different-networks = हो सकता है दोनों लैपटॉप अलग-अलग नेटवर्क पर हों।
doctor-different-networks-fix = दोनों लैपटॉप को एक ही Wi-Fi से जोड़ें।
doctor-guest-network = यह नेटवर्क डिवाइसों को एक-दूसरे को देखने से रोक सकता है, जैसा अक्सर मेहमान, होटल और दफ़्तर के Wi-Fi में होता है।
doctor-guest-network-fix = अपने घर का Wi-Fi या अपने फ़ोन का हॉटस्पॉट इस्तेमाल करें।
doctor-probe-interfaces = PCTwin इस लैपटॉप के नेटवर्क कनेक्शन नहीं देख सका।
doctor-probe-hears-itself = PCTwin जाँच नहीं सका कि यह लैपटॉप नेटवर्क पर मिल सकता है या नहीं।
doctor-probe-local-network = PCTwin Mac की लोकल नेटवर्क सेटिंग नहीं जाँच सका।
doctor-probe-network-profiles = PCTwin नहीं जाँच सका कि Windows इस नेटवर्क को कैसे मानता है।
doctor-hotspot = अब भी काम नहीं कर रहा? अपने फ़ोन का हॉटस्पॉट चालू करें और दोनों लैपटॉप उससे जोड़ें। मूव सीधे दोनों लैपटॉप के बीच होता है, आपके फ़ोन के डेटा से नहीं।
windows-make-other-administrator = पक्का करें कि जो व्यक्ति लैपटॉप रखेगा उसका उस पर एडमिनिस्ट्रेटर खाता हो।
windows-sign-in-other-administrator = साइन आउट करें, फिर उस दूसरे एडमिनिस्ट्रेटर खाते से साइन इन करें। कोई खाता खुद को नहीं हटा सकता।
windows-open-other-users = Settings खोलें, फिर Accounts, फिर Other users।
windows-remove-account = अपना खाता चुनें, फिर Remove।
windows-delete-account-and-data = Delete account and data चुनें। बाकी सबके खाते जैसे हैं वैसे ही रहेंगे।
windows-sign-out-of-everything = पहले पुराने लैपटॉप पर अपने ऐप और खातों से साइन आउट करें।
windows-open-recovery = Settings खोलें, फिर System, फिर Recovery।
windows-reset-this-pc = Reset PC चुनें।
windows-remove-everything = Remove everything चुनें।
windows-clean-the-drive = सेटिंग बदलें ताकि ड्राइव साफ़ की जाए, सिर्फ़ खाली नहीं। इसमें कुछ घंटे लगते हैं।
windows-wait-until-setup-screen = लैपटॉप के सेटअप स्क्रीन दिखाने तक रुकें। तब वह सौंपने के लिए तैयार है।
macos-make-other-administrator = पक्का करें कि जो व्यक्ति Mac रखेगा उसका उस पर एडमिनिस्ट्रेटर खाता हो।
macos-sign-in-other-administrator = लॉग आउट करें, फिर उस दूसरे एडमिनिस्ट्रेटर खाते से लॉग इन करें। कोई खाता खुद को नहीं हटा सकता।
macos-open-users-and-groups = System Settings खोलें, फिर Users & Groups।
macos-delete-user = अपने खाते के पास जानकारी बटन चुनें, फिर Delete User।
macos-delete-home-folder = होम फ़ोल्डर मिटाना चुनें।
macos-sign-out-of-apple-account = पहले पुराने Mac पर अपने Apple खाते से साइन आउट करें, ताकि वह अगले मालिक के लिए खाली हो।
macos-open-transfer-or-reset = System Settings खोलें, फिर General, फिर Transfer or Reset।
macos-erase-all-content-and-settings = Erase All Content and Settings चुनें, और चरणों का पालन करें।
macos-older-mac-use-recovery = अगर आपका Mac यह नहीं देता (पुराने Mac), तो उसे macOS Recovery में शुरू करें, Disk Utility से ड्राइव मिटाएँ, फिर macOS दोबारा इंस्टॉल करें।
macos-wait-until-setup-screen = Mac के सेटअप स्क्रीन दिखाने तक रुकें। तब वह सौंपने के लिए तैयार है।
linux-sign-in-other-administrator = किसी दूसरे एडमिनिस्ट्रेटर खाते से साइन इन करें।
linux-open-users = Settings खोलें, फिर Users।
linux-remove-user-and-files = अपना खाता चुनें, फिर Remove User, और उसकी फ़ाइलें मिटाना चुनें।
linux-no-built-in-reset = Linux में ड्राइव मिटाने वाला अपना रीसेट नहीं होता।
linux-drive-maker-secure-erase = ड्राइव बनाने वाली कंपनी का सुरक्षित मिटाने वाला टूल इस्तेमाल करें, या अगर हो तो लैपटॉप की स्टार्टअप सेटिंग में उसका अपना मिटाने का विकल्प।
linux-reinstall-from-usb = फिर USB स्टिक से नया सिस्टम इंस्टॉल करें।
any-sign-in-other-administrator = पुराने लैपटॉप पर किसी दूसरे एडमिनिस्ट्रेटर खाते से साइन इन करें।
any-use-system-account-settings = सिस्टम की अपनी खाता सेटिंग से अपना खाता हटाएँ।
any-use-system-reset = लैपटॉप का अपना रीसेट इस्तेमाल करें जो ड्राइव मिटाता है। लैपटॉप बनाने वाली कंपनी की मदद में तरीका बताया है।
any-drive-maker-secure-erase = अगर ऐसा न हो, तो ड्राइव बनाने वाली कंपनी का सुरक्षित मिटाने वाला टूल इस्तेमाल करें।

# 2.9 deliverables
guard-undo-removes-if-package = अगर यह इंस्टॉलर पैकेज से आया है तो PCTwin इसे हटा देगा; वरना इसे खुद हटाएँ
guard-undo-keeps-newer = पहले जैसा करने पर नया संस्करण रहेगा: पुराना वापस नहीं आ सकता
compat-outcome-works = जाँचा गया: काम करता है
compat-outcome-works-with-steps = जाँचा गया: आपके कुछ कदमों के साथ काम करता है
compat-outcome-partly = जाँचा गया: कुछ हिस्से काम करते हैं
compat-outcome-not-supported = जाँचा गया: PCTwin इसे यहाँ सेट नहीं कर सकता
compat-outcome-not-tested = अभी जाँचा नहीं गया
compat-aspect-install = इंस्टॉल करना
compat-aspect-data-import = इसका डेटा लाना
compat-aspect-settings = इसकी सेटिंग
compat-aspect-launch = इसे खोलना
compat-aspect-undo = पहले जैसा करना
compat-result-works = काम करता है
compat-result-guided = आपकी मदद से काम करता है
compat-result-not-supported = काम नहीं करता
compat-result-not-applicable = ज़रूरत नहीं
compat-result-not-tested = जाँचा नहीं गया
compat-step-sign-in = साइन इन करें
compat-step-licence = लाइसेंस कुंजी डालें
compat-step-import-in-app = ऐप में अपना डेटा इंपोर्ट करें
compat-step-relink = इसे फिर से अपनी फ़ाइलें दिखाएँ
compat-step-restart = लैपटॉप फिर से चालू करें
compat-how-lab = PCTwin ने साफ़ लैपटॉप पर जाँचा
compat-how-real-laptops = असली लैपटॉप पर जाँचा गया
work-what-linked-files = जुड़ी हुई फ़ाइलें
work-what-font = फ़ॉन्ट
work-what-archive = आर्काइव खुलता है
work-what-database = डेटाबेस खुलता है
work-what-task = कुछ जो आप खुद जाँचते हैं
work-label-machine-checked = PCTwin ने जाँचा
work-label-person-confirmed = आपने जाँचा
work-label-still-needs-attention = अभी जाँच बाकी है
work-state-not-run-yet = अभी जाँचा नहीं
work-state-all-there = सब कुछ है
work-state-missing = कुछ कम है
work-state-damaged = पूरा नहीं है
work-state-not-there = मौजूद नहीं है
work-state-waiting-for-you = आपका इंतज़ार है
work-state-you-said-no = आपने कहा कि यह काम नहीं किया
work-state-outside-project = इसके फ़ोल्डर के बाहर के लिंक: देखे नहीं गए
work-state-could-not-read = पढ़ा नहीं जा सका
work-state-not-supported = PCTwin अभी इस तरह की फ़ाइल नहीं जाँच सकता
work-state-partly-checked = सब कुछ नहीं देखा गया
work-problem-not-chosen = कोई फ़ाइल नहीं चुनी गई।
work-problem-too-many = एक साथ बहुत ज़्यादा जाँचें। पहले कुछ हटाएँ।
work-problem-no-such-check = वह जाँच अब नहीं है।
work-problem-stale = सूची बदल गई। इसे फिर से देखें।
work-problem-bad-name = 200 अक्षरों तक का नाम लिखें।
work-problem-wrong-kind = यह जवाब इस जाँच के लिए नहीं है।
work-problem-failed = कुछ गलत हो गया। फिर से कोशिश करें।
repair-waits-old-laptop = पुराना लैपटॉप जुड़ा होना चाहिए
repair-waits-internet = इंटरनेट चाहिए
repair-waits-administrator = एडमिनिस्ट्रेटर की मंज़ूरी चाहिए
repair-waits-installer-tool = सिस्टम का ऐप इंस्टॉलर चाहिए
repair-waits-space = इस लैपटॉप पर और जगह चाहिए
repair-waits-close-app = ऐप बंद होना चाहिए
repair-problem-nothing-to-repair = चुनी गई किसी चीज़ को ठीक करने की ज़रूरत नहीं है।
repair-problem-stale = सूची बदल गई। इसे फिर से देखें।
repair-problem-busy = PCTwin किसी और काम में व्यस्त है। उसके पूरा होने पर फिर से कोशिश करें।
repair-problem-failed = कुछ गलत हो गया। और कुछ नहीं बदला गया।
changed-why-app-updated = आपकी मंज़ूरी के बाद ऐप अपडेट हुआ
changed-why-app-removed = आपकी मंज़ूरी के बाद ऐप हटाया गया
changed-why-destination-changed = जिस फ़ोल्डर में यह जाता है वह बदल गया
changed-why-project-edited = आपकी मंज़ूरी के बाद इसे पुराने लैपटॉप पर बदला गया
changed-step-install = इंस्टॉल करना
changed-step-settings = सेटिंग
changed-step-app-data = ऐप डेटा
changed-step-files = फ़ाइलें
safe-stop-busy-moving = फ़ाइलें ले जा रहा है
safe-stop-busy-rebuilding = ऐप और सेटिंग सेट कर रहा है
safe-stop-busy-app-data = ऐप डेटा
safe-stop-busy-undoing = एक बार का ले जाना पहले जैसा कर रहा है
kit-part-move-record = आपके ले जाने का रिकॉर्ड
kit-part-results = हर चीज़ का नतीजा
kit-part-before-wipe-list = जो अभी भी सिर्फ़ पुराने लैपटॉप पर है
kit-part-file-data = आपकी फ़ाइलें
kit-part-setup-steps = ऐप फिर से सेट करने के कदम
kit-part-recovery-tool = चीज़ें वापस लाने का टूल

# 2.8 screens
report-group-apps = ऐप
report-group-settings = सेटिंग्स
report-group-special = ऐप डेटा और दूसरी चीज़ें
report-group-people = दूसरे लोग
report-count-apps = { $count ->
        [one] { $count } ऐप
       *[other] { $count } ऐप
    }
report-count-settings = { $count ->
        [one] सेटिंग्स का { $count } समूह
       *[other] सेटिंग्स के { $count } समूह
    }
report-count-special = { $count ->
        [one] { $count } चीज़
       *[other] { $count } चीज़ें
    }
report-count-people = { $count ->
        [one] { $count } व्यक्ति
       *[other] { $count } लोग
    }
report-beyond-done = सब पूरा हुआ और जाँच लिया गया
report-beyond-some = { $total } में से { $done } पूरे हुए और जाँच लिए गए
report-beyond-left = { $count ->
        [one] { $count } नहीं ले जाया गया: आपने इसे छोड़ा
       *[other] { $count } नहीं ले जाए गए: आपने इन्हें छोड़ा
    }
report-beyond-more = { $count ->
        [one] { $count } और यहाँ नहीं दिखाया गया।
       *[other] { $count } और यहाँ नहीं दिखाए गए।
    }
report-here = { $count ->
        [one] { $count } ऐप इस लैपटॉप पर पहले से था। PCTwin ने उसे वैसा ही छोड़ दिया।
       *[other] { $count } ऐप इस लैपटॉप पर पहले से थे। PCTwin ने उन्हें वैसा ही छोड़ दिया।
    }
progress-ready-title = अभी इस्तेमाल के लिए तैयार
progress-ready-body = { $count ->
        [one] { $groups } कॉपी हो गया है और जाँच लिया गया है। बाकी आने तक आप इसे इस लैपटॉप पर खोल सकते हैं।
       *[other] { $groups } कॉपी हो गए हैं और जाँच लिए गए हैं। बाकी आने तक आप इन्हें इस लैपटॉप पर खोल सकते हैं।
    }
finish-strip = अपना स्थानांतरण पूरा करें
finish-title = अपना स्थानांतरण पूरा करें
finish-loading = आपका स्थानांतरण देखा जा रहा है।
finish-retry = फिर कोशिश करें
finish-back-to-list = सूची पर वापस
finish-open = खोलें
finish-open-name = { $title } खोलें
finish-more-not-shown = और जो नहीं दिखाए गए: { $count }
finish-problem-no-move = इस लैपटॉप पर PCTwin के पास पूरा करने के लिए कोई स्थानांतरण नहीं है। कुछ भी नहीं बदला।
finish-problem-changed = जो दिख रहा है वह पुराना है। जवाब देने से पहले इसे फिर से देखें। कुछ भी नहीं बदला।
finish-problem-not-allowed = यह नहीं किया जा सकता। कुछ भी नहीं बदला।
finish-problem-busy = कोई स्थानांतरण या दूसरा चरण चल रहा है। उसके खत्म होने तक रुकें, फिर कोशिश करें। कुछ भी नहीं बदला।
finish-problem-failed = इस लैपटॉप पर कुछ गड़बड़ हुई। कुछ भी नहीं बदला। फिर कोशिश करें।
finish-headline-ready-for-selected-work = आपकी चुनी हुई हर चीज़ इस लैपटॉप पर है
finish-headline-ready-for-selected-work-body = PCTwin को आपकी चुनी हुई हर चीज़ इस लैपटॉप पर मिली और उसने जहाँ तक हो सका जाँची। इसका यह मतलब नहीं कि हर ऐप पहले जैसा चलेगा।
finish-headline-finished-with-tasks = स्थानांतरण पूरा हुआ, कुछ काम बाकी हैं
finish-headline-finished-with-tasks-body = कुछ चीज़ों में आपकी ज़रूरत है, या वे कॉपी नहीं हुईं। वे इसी पेज पर सूचीबद्ध हैं।
finish-headline-data-preserved = सब कुछ यहाँ है, पर कुछ को खोलने के लिए चाबी चाहिए
finish-headline-data-preserved-body = सब कुछ इस लैपटॉप पर है। कुछ चीज़ें खोलने के लिए कुंजी या रिकवरी कोड चाहिए, और PCTwin उसे यहाँ जाँच नहीं सका।
finish-headline-handover-unresolved = पुराना लैपटॉप अभी सौंपा नहीं गया
finish-headline-handover-unresolved-body = यह लैपटॉप तैयार है। पुराना लैपटॉप छोड़ने की सूची में अभी कुछ बातें खुली हैं।
finish-headline-nothing-moved = स्थानांतरण के लिए कुछ नहीं चुना गया
finish-headline-nothing-moved-body = कोई फ़ाइल, ऐप या सेटिंग नहीं चुनी गई, इसलिए कुछ कॉपी नहीं हुआ।
finish-tally-title = क्या स्थानांतरित हुआ
finish-tally-selected = चुनी गईं: { $count }
finish-tally-done = कॉपी और जाँची गईं: { $count }
finish-tally-needs-action = आपकी ज़रूरत वाली: { $count }
finish-tally-failed = कॉपी नहीं हुईं: { $count }
finish-tally-waiting = अभी इंतज़ार में: { $count }
finish-tally-not-reachable = पुराना लैपटॉप नहीं मिला: { $count }
finish-tally-no-result-yet = अभी कोई नतीजा नहीं: { $count }
finish-tally-skipped = आपने छोड़ीं: { $count }
finish-tally-left-out = छोड़ी गईं: { $count }
finish-tally-accepted-loss = जानबूझकर पीछे छोड़ी गईं: { $count }
finish-others = दूसरे लोगों की चीज़ें, सिर्फ़ गिनती, कभी दिखाई नहीं जातीं: { $count }
finish-after-title = अब इस लैपटॉप पर जाँचा गया
finish-after-body = PCTwin ने इन्हें पुराने लैपटॉप पर गिना, फिर इस लैपटॉप पर दोबारा गिना।
finish-after-count = { $atScan } में से { $onNew } इस लैपटॉप पर हैं
finish-after-all = सब मौजूद हैं
finish-after-short = कुछ कम हैं
finish-group-photos = फ़ोटो
finish-group-videos = वीडियो
finish-group-music = संगीत
finish-group-documents = दस्तावेज़
finish-group-other-files = दूसरी फ़ाइलें
finish-group-apps = ऐप
finish-group-settings = सेटिंग
finish-note-parental-controls = पैरेंटल कंट्रोल और परिवार की सुरक्षा सेटिंग नहीं जातीं। उन्हें इस लैपटॉप पर फिर से सेट करें।
finish-note-not-everywhere = पुराने लैपटॉप की कुछ जगहों में खोज नहीं की गई।
finish-note-results-refused = कुछ नतीजे नहीं माने गए, क्योंकि इस लैपटॉप ने उन्हें पूरा हुआ नहीं दिखाया।
finish-note-undo-open = इस स्थानांतरण के लिए पहले जैसा करना अभी खुला है। पुराना लैपटॉप मिटाने पर यह हमेशा के लिए बंद हो जाता है।
finish-open-title = पूरा नहीं हुआ
finish-open-body = आपकी ये चीज़ें अभी ध्यान माँगती हैं।
finish-standing-copied = कॉपी हुई
finish-standing-reinstalled = दोबारा इंस्टॉल हुई
finish-standing-converted = बदली गई
finish-standing-needs-action = आपकी ज़रूरत है
finish-standing-failed = कॉपी नहीं हुई
finish-standing-waiting = इंतज़ार में
finish-standing-not-reachable = पुराना लैपटॉप नहीं मिला
finish-standing-no-result-yet = अभी कोई नतीजा नहीं
finish-standing-skipped-by-user = आपने छोड़ी
finish-standing-left-out = छोड़ी गई
finish-access-key-not-checked = इसे खोलने की कुंजी या कोड यहाँ जाँचा नहीं गया।
finish-access-not-on-new-laptop = यह इस लैपटॉप पर नहीं है।
finish-search-title = PCTwin ने पुराने लैपटॉप पर कहाँ देखा
finish-search-searched = इनमें देखा: { $places }
finish-search-not-searched = इनमें नहीं देखा: { $places }
finish-search-all = PCTwin ने अपनी जानी हुई हर जगह में देखा। वह यह नहीं कह सकता कि कहीं और कुछ नहीं है।
finish-old-checked = पुराना लैपटॉप आखिरी बार यहाँ जाँचा गया: { $when }
finish-old-never = स्थानांतरण के बाद से पुराना लैपटॉप जाँचा नहीं गया।
finish-steps-title = अगले कदम
finish-step-setup = सेटअप पूरा करें
finish-step-setup-detail = वे ऐप जिनमें आपको साइन इन करना है या कोई चरण पूरा करना है।
finish-step-goodbye = पुराना लैपटॉप छोड़ने से पहले
finish-step-goodbye-detail = जो अभी सिर्फ़ पुराने लैपटॉप पर है।
finish-step-catchup = हाल के बदलाव कॉपी करें
finish-step-catchup-detail = स्थानांतरण के बाद पुराने लैपटॉप पर जो बदला, उसे कॉपी करें।
finish-step-handover = पुराना लैपटॉप सौंपें
finish-step-handover-detail = सिस्टम के अपने टूल से अपना खाता हटाएँ या पुराना लैपटॉप मिटाएँ।
finish-step-undo = इस स्थानांतरण को पहले जैसा करें
finish-step-undo-detail = ऐप हटाएँ और सेटिंग वापस रखें।
finish-step-cleanup = सफ़ाई करें
finish-step-cleanup-detail = इस लैपटॉप से PCTwin की अस्थायी फ़ाइलें हटाएँ।
finish-step-report = रिपोर्ट साझा करें
finish-step-report-detail = एक सादी रिपोर्ट सहेजें। उसमें क्या हो, यह आप चुनते हैं।
finish-step-doctor = कनेक्शन जाँचें
finish-step-doctor-detail = पता लगाएँ कि दोनों लैपटॉप क्यों नहीं जुड़ पा रहे।
finish-setup-title = सेटअप पूरा करें
finish-setup-body = इन ऐप में आपका एक कदम चाहिए। टिक वही दर्ज करता है जो आप कहते हैं। यह PCTwin की जाँच नहीं गिना जाता।
finish-setup-empty = आपके लिए कुछ इंतज़ार में नहीं है।
finish-setup-sign-in = { $name } में साइन इन करें
finish-setup-licence-key = { $name } की लाइसेंस कुंजी डालें
finish-setup-set-up-again = { $name } को फिर से सेट करें
finish-setup-download-from-maker = { $name } को उसके बनाने वाले की वेबसाइट से डाउनलोड करें
finish-setup-permission = { $name } जब अनुमति माँगे, तब दें
finish-setup-restart = { $name } पूरा करने के लिए इस लैपटॉप को रीस्टार्ट करें
finish-setup-finish-by-hand = { $name } को खुद पूरा करें
finish-setup-parts-missing = { $name } के कुछ हिस्से नहीं पहुँचे। उन्हें फिर से भेजें
finish-setup-export-first = पहले पुराने लैपटॉप पर { $name } से एक्सपोर्ट करें
finish-setup-relink = { $name } को अपने खाते से फिर से जोड़ें
finish-setup-look-at-it = { $name } को देखें और जाँचें कि सब ठीक है
finish-setup-mark-open = पूरा नहीं हुआ
finish-setup-mark-machine-checked = PCTwin ने जाँचा
finish-setup-mark-you-did-it = आपने कहा कि हो गया
finish-setup-tick-hint = कर लेने पर टिक करें।
finish-setup-save-row = अपनी ऐप सूची और सेटअप सहेजें
finish-setup-save-row-detail = अपने ऐप और सेटिंग की सूची मेरा PCTwin इतिहास में रखें।
finish-save-title = अपनी ऐप सूची और सेटअप सहेजें
finish-save-body = PCTwin आपके ऐप और सेटिंग की सूची मेरा PCTwin इतिहास में रखता है, सिर्फ़ इस लैपटॉप पर। यह सूची है, आपकी फ़ाइलों की कॉपी नहीं।
finish-save-button = सूची सहेजें
finish-save-saved = मेरा PCTwin इतिहास में सहेजा गया। ऐप: { $apps }। सेटिंग: { $settings }।
finish-save-earlier-title = पहले सहेजी गई
finish-save-earlier = सहेजी गई: { $when }
finish-goodbye-title = पुराना लैपटॉप छोड़ने से पहले
finish-goodbye-body = यह वह है जो अभी सिर्फ़ पुराने लैपटॉप पर है। हर एक का जवाब दें। यहाँ कुछ मिटाया नहीं जाता। मिटाना अलग चरण है।
finish-goodbye-saved = यह सूची इस लैपटॉप पर सहेजी गई है।
finish-goodbye-only-on-old-laptop = सिर्फ़ पुराने लैपटॉप पर: { $count }
finish-goodbye-key-not-checked = ऐसी चीज़ें जिनकी कुंजी या कोड जाँचा नहीं गया: { $count }
finish-goodbye-tied-to-old-laptop = पुराने लैपटॉप से बँधी: { $count }
finish-goodbye-do-first-on-old-laptop = पहले पुराने लैपटॉप पर करने के लिए: { $count }
finish-goodbye-steps-not-finished = अधूरे चरण: { $count }
finish-goodbye-not-searched = जिन जगहों में नहीं देखा गया: { $count }
finish-goodbye-old-laptop-not-reachable = पुराना लैपटॉप नहीं मिल सका
finish-goodbye-special-sign-in = { $names }: फिर से साइन इन करें
finish-goodbye-special-relink = { $names }: इसे अपने खाते से फिर से जोड़ें
finish-goodbye-special-export-first = { $names }: पहले पुराने लैपटॉप से ज़रूरी चीज़ें एक्सपोर्ट करें
finish-goodbye-special-look-at-it = { $names }: इसे देखें और जाँचें कि सब ठीक है
finish-goodbye-places = जगहें: { $places }
finish-goodbye-did-it = मैंने कर लिया
finish-goodbye-accept = मैं मानता/मानती हूँ कि यह खो जाएगा
finish-goodbye-accept-warning = अगर आप यह मानते हैं, तो यह सिर्फ़ पुराने लैपटॉप पर रहेगा। पुराना लैपटॉप मिटाने पर यह चला जाएगा।
finish-goodbye-cannot-answer = इसका फ़ैसला सिर्फ़ पुराने लैपटॉप की नई जाँच से हो सकता है। उसे जोड़ें और फिर जाँचें।
finish-goodbye-res-open = अभी जवाब नहीं दिया
finish-goodbye-res-person-did-it = आपने कहा कि कर लिया
finish-goodbye-res-accepted-loss = आपने माना कि यह खो जाएगा
finish-goodbye-answer-label = आपका जवाब: { $title }
finish-goodbye-keep-title = पुराने लैपटॉप को इस तारीख तक वैसा ही रखें
finish-goodbye-keep-body = सुझाई गई तारीख: { $date }। तब तक इसे न मिटाएँ, ताकि कुछ चाहिए हो तो मिल सके।
finish-goodbye-keep-label = इस तारीख तक रखें
finish-goodbye-keep-use = सुझाई गई तारीख लें
finish-goodbye-keep-save = यह तारीख सहेजें
finish-goodbye-keep-set = इस तारीख तक रखा गया: { $date }
finish-goodbye-keep-none = अभी कोई तारीख नहीं चुनी
finish-goodbye-blocked = पुराना लैपटॉप अभी सौंपा नहीं जा सकता। अभी खुले: { $count }।
finish-goodbye-blocked-unchecked = स्थानांतरण के बाद से पुराना लैपटॉप जाँचा नहीं गया। पहले उसे फिर से जाँचें।
finish-goodbye-clear = सबका जवाब दे दिया गया है और पुराना लैपटॉप जाँचा गया है। आप सौंपने का अनुरोध कर सकते हैं। वह अलग चरण है।
finish-goodbye-handover = पुराना लैपटॉप सौंपें
finish-goodbye-check-again = पुराना लैपटॉप फिर से जाँचें
finish-catchup-title = हाल के बदलाव कॉपी करें
finish-catchup-body = देखें कि स्थानांतरण के बाद पुराने लैपटॉप पर क्या बदला। हर बदलाव आप देखते हैं। PCTwin सिर्फ़ वही भेजता है जो आप चुनें।
finish-catchup-purpose-check = सिर्फ़ पुराने लैपटॉप को देखें
finish-catchup-purpose-check-detail = कुछ कॉपी नहीं होता। छोड़ने से पहले की आपकी सूची अपडेट होती है।
finish-catchup-purpose-catch-up = देखें और जो बदला उसे कॉपी करें
finish-catchup-purpose-catch-up-detail = आप पहले बदलाव देखते हैं और चुनते हैं कि क्या भेजा जाए।
finish-catchup-armed = तैयार। पुराने लैपटॉप को वैसे ही जोड़ें जैसे स्थानांतरण के लिए जोड़ा था। पुराना लैपटॉप जवाब दे तो यह पेज बदलाव दिखाएगा।
finish-catchup-connect = पुराना लैपटॉप जोड़ें
finish-catchup-waiting = पुराने लैपटॉप का इंतज़ार है।
finish-catchup-unreadable = PCTwin पुराने लैपटॉप का जवाब पढ़ नहीं सका। कुछ भी नहीं बदला। फिर कोशिश करें।
finish-catchup-changes-title = क्या बदला
finish-catchup-check-only = कुछ नहीं भेजा जाता। यह सूची सिर्फ़ इतना अपडेट करती है कि PCTwin पुराने लैपटॉप के बारे में क्या जानता है।
finish-catchup-none = पुराने लैपटॉप पर कुछ नहीं बदला।
finish-catchup-others = दूसरे लोगों की चीज़ों में बदलाव, सिर्फ़ गिनती: { $count }
finish-catchup-choose-label = PCTwin { $title } के साथ क्या करे?
finish-catchup-send-label = { $title } भेजें
finish-catchup-nothing-to-send = भेजने के लिए कुछ नहीं
finish-change-new = पुराने लैपटॉप पर नया
finish-change-updated = पुराने लैपटॉप पर बदला
finish-change-both-changed = दोनों लैपटॉप पर बदला
finish-change-gone-on-new-laptop = इस लैपटॉप से हटाया गया
finish-change-gone-on-old-laptop = पुराने लैपटॉप से हटाया गया
finish-change-cannot-tell = PCTwin बता नहीं सकता
finish-choice-none = न भेजें
finish-choice-send = फिर से भेजें
finish-choice-keep-both = दोनों कॉपी रखें
finish-choice-keep-new-laptop-version = इस लैपटॉप वाली रखें
finish-catchup-approve = मेरे चुने बदलाव भेजें: { $count }
finish-catchup-sent = कॉपी करने के लिए भेजा गया। सिर्फ़ आपके चुने बदलाव कॉपी होते हैं।
finish-handover-title = पुराना लैपटॉप सौंपें
finish-handover-body = हर विकल्प अलग फ़ैसला है। अपने पेज पर पुष्टि करने तक कुछ नहीं होता।
finish-handover-remove-title = पुराने लैपटॉप से मेरा खाता हटाएँ
finish-handover-remove-detail = सिर्फ़ आपका खाता। दूसरों के खाते रहेंगे।
finish-handover-erase-title = पुराना लैपटॉप मिटाएँ
finish-handover-erase-detail = पुराने लैपटॉप की ड्राइव पर सब कुछ।
finish-handover-preview-remove = मेरा खाता हटाएँ
finish-handover-preview-erase = पुराना लैपटॉप मिटाएँ
finish-handover-removes = क्या हटेगा
finish-handover-keeps = क्या रहेगा
finish-handover-keeps-none = उस पर कुछ नहीं रखा जाता।
finish-handover-route = यह कैसे होता है
finish-handover-route-body = PCTwin यह खुद नहीं करता। यह सिस्टम का अपना टूल करता है, और PCTwin आपको उसके पास पहुँचा देता है।
finish-handover-continue = पुष्टि के लिए आगे बढ़ें
finish-handover-not-offered = यह अभी उपलब्ध नहीं है
finish-handover-refused-open = छोड़ने से पहले की आपकी सूची में अभी खुली बातें: { $count }
finish-handover-refused-nothing-done = कुछ नहीं किया गया।
finish-handover-to-list = छोड़ने से पहले की सूची पर जाएँ
finish-route-system-windows = Windows
finish-route-system-macos = macOS
finish-route-system-linux = Linux
finish-route-unknown-step = सिस्टम जो अगला चरण दिखाए, उसे करें।
finish-hnote-ssd-not-secure-wipe = SSD पर फ़ाइलें हटाना सुरक्षित मिटाना नहीं है।
finish-hnote-does-not-cancel-subscriptions = खाता हटाने से सदस्यताएँ रद्द नहीं होतीं और आप दूसरी जगहों से साइन आउट नहीं होते।
finish-hnote-others-stay = बाकी सबके खाते जैसे हैं वैसे रहेंगे।
finish-hnote-cannot-be-undone = इसे पहले जैसा नहीं किया जा सकता।
finish-hnote-closes-undo = आगे बढ़ने पर इस स्थानांतरण का पहले जैसा करना हमेशा के लिए बंद हो जाता है।
finish-hnote-sign-in-as-another-administrator = कोई खाता खुद को नहीं हटा सकता। पहले पुराने लैपटॉप पर किसी दूसरे एडमिन खाते से साइन इन करें।
finish-hnote-family-group-first = अगर खाता Microsoft परिवार समूह में है, तो पहले उसे ऑनलाइन परिवार से हटाएँ।
finish-hnote-not-a-certificate = PCTwin दर्ज करता है कि आपने क्या माँगा और आपने उसे क्या बताया। यह इसका प्रमाणपत्र नहीं कि कुछ मिटाया गया।
finish-hnote-encrypt-first-helps = रीसेट से पहले ड्राइव का एन्क्रिप्शन चालू करने से जो बचता है उसे पढ़ना बहुत मुश्किल हो जाता है।
finish-refused-wrong-device = यह चरण इस लैपटॉप पर उपलब्ध नहीं है।
finish-refused-undo-not-closed = इस स्थानांतरण का पहले जैसा करना अभी खुला है। पहले इसे बंद करना होगा।
finish-refused-old-laptop-not-checked = पुराने लैपटॉप को आख़िरी बार 15 मिनट से ज़्यादा पहले जाँचा गया था। पहले उसे दोबारा जाँचें।
finish-refused-list-out-of-date = छोड़ने से पहले की सूची पुरानी है। उसे फिर से खोलें।
finish-refused-list-not-clear = छोड़ने से पहले की सूची में अभी खुली बातें हैं।
finish-refused-no-write-exclusion = PCTwin पक्का नहीं कर सका कि पुराने लैपटॉप पर अभी कुछ बदल नहीं रहा।
finish-refused-no-such-account = पुराने लैपटॉप पर आपका खाता नहीं मिला।
finish-refused-other-people-use-it = इस खाते को दूसरे लोग भी इस्तेमाल करते हैं, इसलिए PCTwin इसे नहीं हटाएगा।
finish-refused-not-qualified = इस पुराने लैपटॉप को इस तरह नहीं सौंपा जा सकता।
finish-refused-no-fresh-confirmation = नई पुष्टि चाहिए।
finish-refused-wrong-confirmation = वह पुष्टि अभी दिख रही बातों से मेल नहीं खाती। फिर से देखें।
finish-refused-changed-since-confirmed = आपके पुष्टि करने के बाद कुछ बदल गया। फिर से देखें और दोबारा पुष्टि करें।
finish-refused-expired = पुष्टि की अवधि खत्म हो गई। फिर से देखें और दोबारा पुष्टि करें।
finish-refused-already-used = यह पुष्टि पहले ही इस्तेमाल हो चुकी है।
finish-refused-nothing-to-remove = हटाने के लिए कुछ नहीं है।
finish-confirm-title-remove-my-account = पुष्टि करें: मेरा खाता हटाएँ
finish-confirm-title-erase-old-laptop = पुष्टि करें: पुराना लैपटॉप मिटाएँ
finish-confirm-target = इसमें बदलाव होगा: { $target }
finish-confirm-consequences = इसका मतलब
finish-confirm-tick-remove-my-account = मैं समझता/समझती हूँ कि खाता { $target } पुराने लैपटॉप से हट जाएगा और इसे पहले जैसा नहीं किया जा सकता।
finish-confirm-tick-erase-old-laptop = मैं समझता/समझती हूँ कि { $target } पर सब कुछ मिट जाएगा और इसे पहले जैसा नहीं किया जा सकता।
finish-confirm-go = पुष्टि करें और सिस्टम का टूल खोलें
finish-confirm-cancel = वापस जाएँ और कुछ न करें
finish-confirm-needs-tick = ऊपर का बॉक्स टिक करने पर बटन काम करेगा।
finish-started-title = सिस्टम के अपने चरण अपनाएँ
finish-started-body = नीचे के चरण सिस्टम के हैं, PCTwin के नहीं। PCTwin उनका अपना कोई रूप नहीं दिखाता।
finish-started-opened = PCTwin ने सिस्टम का पेज खोल दिया।
finish-started-not-opened = PCTwin सिस्टम का पेज नहीं खोल सका। उसे खुद खोलें और चरणों का पालन करें।
finish-started-undo-closed = इस स्थानांतरण का पहले जैसा करना अब बंद है।
finish-says-title = PCTwin को बताएँ कि क्या हुआ
finish-says-label = आपने क्या देखा?
finish-says-choose = चुनें कि आपने क्या देखा
finish-says-done = मैंने चरण पूरे किए
finish-says-partly = मैंने कुछ चरण पूरे किए
finish-says-cancelled-part-way = मैंने बीच में रद्द किया
finish-says-not-started = मैंने शुरू नहीं किया
finish-says-cannot-tell = मैं बता नहीं सकता/सकती
finish-says-record = जो देखा उसे दर्ज करें
finish-recorded-person-says-done = दर्ज किया: आपने कहा कि यह पूरा हुआ। यह आपकी बात है। यह प्रमाणपत्र नहीं है।
finish-recorded-partial = दर्ज किया: इसका सिर्फ़ एक हिस्सा हुआ।
finish-recorded-interrupted = दर्ज किया: यह बीच में रुक गया।
finish-recorded-cancelled-after-some-effects = दर्ज किया: कुछ बदलाव होने के बाद इसे रद्द किया गया।
finish-recorded-not-started = दर्ज किया: यह शुरू नहीं हुआ।
finish-recorded-inconclusive = दर्ज किया: आप बता नहीं सके कि क्या हुआ।
finish-recorded-time = दर्ज किया: { $when }
finish-undo-title = पहले जैसा करने से पहले देखें
finish-undo-body = पहले जैसा करना क्या करता है और क्या छोड़ता है, यह यहाँ है। पहले जैसा करें शुरू करने तक कुछ नहीं होता।
finish-undo-row-removes = PCTwin के इंस्टॉल किए ऐप: पहले जैसा करने पर हट जाते हैं
finish-undo-row-yours = जो ऐप अब आपके हैं: पहले जैसा करना उन्हें छोड़ देता है। चाहें तो खुद हटा लें।
finish-undo-row-settings = जैसी थीं वैसी वापस रखी सेटिंग: { $count }
finish-undo-row-appdata = इन ऐप का डेटा वापस रखा गया
finish-undo-row-files = स्थानांतरण में कॉपी हुई फ़ाइलें: { $count }
finish-undo-limit-files = सिर्फ़ तब जब पुराना लैपटॉप जुड़ा हो, और सिर्फ़ वे कॉपी जिन्हें तब से किसी ने नहीं बदला।
finish-undo-limit-apps = सिर्फ़ तब जब ऐप अब भी वही संस्करण हो जो PCTwin ने इंस्टॉल किया।
finish-undo-keeps = पहले जैसा करना क्या रखता है: जो ऐप अब आपके हैं, और स्थानांतरण के बाद आपने जो बदला।
finish-undo-closed = इस स्थानांतरण का पहले जैसा करना बंद है। इसे इस्तेमाल नहीं किया जा सकता।
finish-undo-nothing = इस स्थानांतरण में पहले जैसा करने के लिए कोई ऐप या सेटिंग नहीं है।
finish-undo-start = पहले जैसा करना शुरू करें
finish-cleanup-title = सफ़ाई करें
finish-cleanup-body = हर सवाल अलग से पूछा जाता है। आपकी पुष्टि के बिना कुछ नहीं हटता।
finish-cleanup-temp = PCTwin की अस्थायी फ़ाइलें
finish-cleanup-temp-detail = वे फ़ाइलें जो PCTwin ने काम करते समय इस लैपटॉप पर बनाईं।
finish-cleanup-model = PCTwin का मॉडल और सहेजा डेटा
finish-cleanup-model-detail = सहायक फ़ाइलें जो PCTwin इस लैपटॉप पर रखता है ताकि वह तेज़ चले।
finish-cleanup-look = देखें
finish-cleanup-removes = हटाया जाएगा
finish-cleanup-keeps = रखा जाएगा
finish-cleanup-keeps-body = ये सिर्फ़ PCTwin की जैसी दिखती हैं, या इस्तेमाल में हैं।
finish-cleanup-nothing = हटाने के लिए कुछ नहीं है।
finish-cleanup-tick = हटाया जाएगा के नीचे की चीज़ें हटाएँ
finish-cleanup-confirm = इन्हें हटाएँ
finish-cleanup-done = हटाई गईं: { $removed }। रखी गईं: { $kept }। हटा नहीं सकीं: { $failed }।
finish-doctor-title = कनेक्शन जाँचें
finish-doctor-body = PCTwin इस लैपटॉप को देखकर पता लगाता है कि दोनों लैपटॉप क्यों नहीं जुड़ पा रहे।
finish-doctor-role-label = यह कौन सा लैपटॉप है?
finish-doctor-role-old = पुराना लैपटॉप
finish-doctor-role-new = नया लैपटॉप
finish-doctor-run = जाँच चलाएँ
finish-doctor-running = इस लैपटॉप की जाँच हो रही है।
finish-doctor-found = PCTwin को क्या मिला
finish-doctor-none = PCTwin को इस लैपटॉप पर कोई समस्या नहीं मिली। इसका मतलब यह नहीं कि कनेक्शन चलेगा ही।
finish-doctor-sure = पक्का
finish-doctor-likely = संभव
finish-doctor-fix = क्या करें
finish-doctor-not-checked = PCTwin जाँच नहीं सका
finish-notchecked-unknown = इस लैपटॉप पर एक जाँच नहीं हो सकी।
finish-report-title = रिपोर्ट साझा करें
finish-report-body = चुनें कि रिपोर्ट में क्या हो। आप जो न चुनें, वह शामिल नहीं होता। कुछ भेजा नहीं जाता। आप पढ़ा हुआ पाठ सहेजते हैं और खुद साझा करते हैं।
finish-report-choices = रिपोर्ट में शामिल करें
finish-report-file-names = फ़ाइलों के नाम
finish-report-app-names = ऐप के नाम
finish-report-account-id = खाता आईडी
finish-report-preview = रिपोर्ट जैसी सहेजी जाएगी
finish-report-withheld = रिपोर्ट से बाहर रखा गया: नाम { $names }, गोपनीय बातें { $secrets }, खाता आईडी { $ids }।
finish-report-save = रिपोर्ट सहेजें
finish-report-saved = यहाँ सहेजी गई: { $path }
finish-report-not-saved = कोई फ़ाइल नहीं सहेजी गई।
finish-report-feedback = PCTwin टीम को बताने के लिए कि क्या हुआ, मदद खोलें, फिर फ़ीडबैक। खाता नहीं चाहिए, और अपने आप कुछ नहीं भेजा जाता।
finish-report-help = मदद खोलें
finish-report-text-title = PCTwin स्थानांतरण रिपोर्ट
finish-report-text-unnamed-file = एक फ़ाइल
finish-report-text-unnamed-app = एक ऐप
finish-report-text-account = खाता आईडी: { $id }
finish-report-text-others = दूसरे लोगों की अधूरी चीज़ें, सिर्फ़ गिनती: { $count }
finish-report-text-goodbye = छोड़ने से पहले की सूची: { $total } बातें, { $open } अभी खुली
finish-report-text-lines = अधूरी चीज़ें
rescue-eyebrow = लैपटॉप से फ़ाइलें बचाएँ
rescue-title = लैपटॉप से फ़ाइलें बचाएँ
rescue-lead = जो लैपटॉप चालू नहीं होता, उससे फ़ाइलें वापस पाएँ। उसकी ड्राइव इस लैपटॉप से जोड़ें, फिर उसे यहाँ चुनें।
rescue-checking = जुड़ी हुई ड्राइव ढूँढी जा रही हैं।
rescue-failed = PCTwin इस लैपटॉप से जुड़ी ड्राइव की सूची नहीं बना सका। केबल देखें, फिर दोबारा जाँचें।
rescue-empty = कोई ड्राइव नहीं जुड़ी है। पुराने लैपटॉप की ड्राइव को USB अडैप्टर से जोड़ें, फिर दोबारा जाँचें।
rescue-looks-like-laptop = लैपटॉप की ड्राइव लगती है
rescue-choose = { $name } चुनें
rescue-check-again = दोबारा जाँचें
rescue-locked-title = अगर ड्राइव लॉक है
rescue-locked-windows = अगर ड्राइव BitLocker से लॉक है, तो ड्राइव खोलने पर Windows अपनी विंडो में उसकी रिकवरी कुंजी माँगता है।
rescue-wait-title = { $drive } पढ़ी जा रही है
rescue-wait-body = PCTwin { $drive } के फ़ोल्डर देख रहा है। ड्राइव पर कुछ नहीं बदला जाता।
rescue-accept-title = { $drive } पर मिली फ़ाइलें
rescue-accept-body = स्वीकार करें, और PCTwin उन्हें इस लैपटॉप पर कॉपी करेगा। ड्राइव पर कुछ नहीं बदला जाता।
rescue-progress-title = { $drive } से फ़ाइलें कॉपी हो रही हैं
rescue-safe = ड्राइव को सिर्फ़ पढ़ा जाता है। उस पर कुछ नहीं बदला जाता।
rescue-outcome-title = { $drive } से कॉपी की गई फ़ाइलें
plan-net-title = इंटरनेट की ज़रूरत किसे है
plan-net-none = इस स्थानांतरण में किसी चीज़ को इंटरनेट की ज़रूरत नहीं है।
plan-net-downloads =
    { $count ->
        [one] PCTwin इंटरनेट से { $count } ऐप डाउनलोड करता है।
       *[other] PCTwin इंटरनेट से { $count } ऐप डाउनलोड करता है।
    }
plan-net-sizes = हर डाउनलोड का आकार डाउनलोड होने के बाद ही पता चलता है।
plan-net-signins =
    { $count ->
        [one] आप ऑनलाइन { $count } चीज़ में फिर से साइन इन करेंगे।
       *[other] आप ऑनलाइन { $count } चीज़ों में फिर से साइन इन करेंगे।
    }
plan-room-title = आपकी फ़ाइलों के अलावा चाहिए जगह
plan-room-body = सबसे व्यस्त पल में PCTwin को आपकी फ़ाइलों के अलावा ऐप के लिए लगभग { $installs } और ऐप के डेटा के लिए लगभग { $appData } चाहिए।
compat-strip = क्या जाँचा गया
compat-title = { $name }: क्या जाँचा गया
compat-open = ब्योरा
compat-open-name = { $name } के लिए जाँच का ब्योरा देखें
compat-list-title =
    { $count ->
        [one] { $count } ऐप की जाँच कैसी रही
       *[other] { $count } ऐप की जाँच कैसी रही
    }
compat-aspects-title = हर हिस्सा
compat-steps-title = आप क्या करेंगे
compat-steps-none = आपको कुछ नहीं करना है।
compat-never-checked = इस ऐप को अभी तक किसी ने जाँचा नहीं है। PCTwin इसे आज़माएगा और बताएगा कि क्या हुआ।
compat-checked-on = { $date } को जाँचा गया।
compat-version = जाँचा गया संस्करण: { $version }
work-strip = काम की जाँच
work-title = जाँचें कि आप यहाँ काम कर सकते हैं
work-body = इस लैपटॉप पर जिन फ़ाइलों, फ़ॉन्ट और कामों की आपको ज़रूरत है, उन्हें जोड़ें। PCTwin वही देखता है जो वह पढ़ सकता है। बाकी के लिए आप बताते हैं कि चला या नहीं। कुछ खोला या चलाया नहीं जाता।
work-empty = अभी कोई जाँच नहीं है। नीचे कोई फ़ाइल, फ़ॉन्ट या काम जोड़ें।
work-looked-at = { $of } में से { $checked } देखे गए।
work-missing = कमी:
work-and-more =
    { $count ->
        [one] और { $count } और।
       *[other] और { $count } और।
    }
work-not-looked-at = नहीं देखा गया:
work-answer-worked = चल गया
work-answer-worked-name = चल गया: { $name }
work-answer-not = नहीं चला
work-answer-not-name = नहीं चला: { $name }
work-remove = हटाएँ
work-remove-name = { $name } हटाएँ
work-run = अभी जाँचें
work-add-title = जाँच जोड़ें
work-add-file = फ़ाइल चुनें
work-add-file-hint = कोई प्रोजेक्ट, आर्काइव या डेटाबेस। PCTwin उसे पढ़ता है और कभी बदलता नहीं।
work-font-label = नाम से कोई फ़ॉन्ट
work-add-font = फ़ॉन्ट जोड़ें
work-task-label = कोई काम जो आप खुद करते हैं
work-task-hint = जैसे: बोर्डिंग पास प्रिंट करना। आप बताते हैं कि चला या नहीं।
work-add-task = काम जोड़ें
repair-strip = जो पूरा नहीं हुआ उसे ठीक करें
repair-title =
    { $count ->
        [one] इस आइटम को ठीक करें
       *[other] इन { $count } आइटम को ठीक करें
    }
repair-title-loading = जो पूरा नहीं हुआ उसे ठीक करें
repair-body =
    { $kept ->
        [one] सिर्फ़ नीचे के आइटम दोबारा आज़माए जाते हैं। { $kept } आइटम, जो पहले चल चुका था, जैसा है वैसा रहता है।
       *[other] सिर्फ़ नीचे के आइटम दोबारा आज़माए जाते हैं। { $kept } आइटम, जो पहले चल चुके थे, जैसे हैं वैसे रहते हैं।
    }
repair-nothing = कुछ ठीक करने को नहीं है।
repair-can-now = अभी आज़माया जा सकता है
repair-waiting = इंतज़ार में
repair-fixed =
    { $count ->
        [one] { $count } आइटम ठीक हुआ।
       *[other] { $count } आइटम ठीक हुए।
    }
repair-run =
    { $count ->
        [one] { $count } आइटम फिर आज़माएँ
       *[other] { $count } आइटम फिर आज़माएँ
    }
changed-strip = आपकी मंज़ूरी के बाद के बदलाव
changed-title =
    { $count ->
        [one] आपकी मंज़ूरी के बाद { $count } चीज़ बदली
       *[other] आपकी मंज़ूरी के बाद { $count } चीज़ें बदलीं
    }
changed-body = सिर्फ़ इन्हें फिर से आपकी मंज़ूरी चाहिए। बाकी सब मंज़ूर रहता है।
changed-affects = असर: { $steps }
changed-held =
    { $count ->
        [one] { $count } चरण आपके जवाब का इंतज़ार कर रहा है।
       *[other] { $count } चरण आपके जवाब का इंतज़ार कर रहे हैं।
    }
changed-leave-out = इन्हें छोड़ दें
changed-approve = फिर से मंज़ूर करें
safe-stop-strip = अपडेट
safe-stop-title = सुरक्षित पल का इंतज़ार
safe-stop-body = PCTwin स्थानांतरण के बीच में खुद को अपडेट या हटाता नहीं है। वह तब तक रुकता है जब तक कुछ भी अधूरा न हो।
safe-stop-failed = PCTwin पता नहीं लगा सका कि यह सुरक्षित है या नहीं। फिर कोशिश करें।
safe-stop-ready = कुछ भी बीच में बदल नहीं रहा। PCTwin अब अपडेट हो सकता है या हटाया जा सकता है।
safe-stop-waiting-for = वह इनके पूरा होने का इंतज़ार कर रहा है:
safe-stop-nothing-going-on = कोई स्थानांतरण, सेटअप या पहले जैसा करना नहीं चल रहा।
safe-stop-check = फिर जाँचें
# Help feedback, safe stop links, catch-up and repair routes
feedback-strip = फ़ीडबैक
feedback-title = स्थानांतरण कैसा रहा?
feedback-body = फ़ीडबैक वैकल्पिक है और मुफ़्त स्थानांतरण पर असर नहीं डालता।
feedback-rating-label = रेटिंग
feedback-rating-none = कोई रेटिंग नहीं
feedback-rating-very-good = बहुत अच्छा
feedback-rating-good = अच्छा
feedback-rating-needs-work = सुधार चाहिए
feedback-words-label = इसे और बेहतर क्या बनाएगा?
feedback-words-hint = वैकल्पिक। नाम, पासवर्ड और फ़ाइलों के नाम न लिखें।
feedback-preview-title = सिर्फ़ यही साझा किया जाएगा
feedback-preview-rating = रेटिंग: { $rating }
feedback-preview-empty = अभी कुछ नहीं। रेटिंग चुनें या कुछ शब्द लिखें।
feedback-sends-nothing = PCTwin अपने आप कुछ नहीं भेजता। खाता नहीं चाहिए। इसे साझा करने के लिए इसे कॉपी करें, फिर PCTwin टीम तक पहुँचने के लिए getpctwin.com खोलें।
feedback-copy = फ़ीडबैक कॉपी करें
feedback-copied = फ़ीडबैक कॉपी हो गया।
feedback-copy-failed = फ़ीडबैक कॉपी नहीं हो सका। ऊपर का टेक्स्ट चुनें और ख़ुद कॉपी करें।
feedback-open-site = getpctwin.com खोलें
help-home-feedback = फ़ीडबैक
help-home-update = PCTwin अपडेट करें या हटाएँ
safe-stop-see = देखें किसका इंतज़ार है
history-repair = आइटम ठीक करें
kit-strip = रिकवरी किट
kit-title = रिकवरी किट सहेजें
kit-body = रिकवरी किट इस स्थानांतरण का एक छोटा रिकॉर्ड है, जो आपके चुने फ़ोल्डर में सहेजा जाता है, जैसे USB ड्राइव पर। इस लैपटॉप के बिना भी यह दिखाता है कि क्या हुआ और क्या बाकी रहा।
kit-lacks-sentence = किट में आपकी फ़ाइलें खुद नहीं होतीं। वह खोई हुई फ़ाइलें वापस नहीं ला सकता।
kit-not-saved = कोई फ़ोल्डर नहीं चुना गया। कुछ सहेजा नहीं गया।
kit-saved-to = यहाँ सहेजा गया:
kit-files-title = किट की फ़ाइलें
kit-holds-title = किट में क्या है
kit-lacks-title = किट में क्या नहीं है
kit-save = फ़ोल्डर चुनें और सहेजें
kit-save-again = फिर सहेजें
kit-history = रिकवरी किट
finish-step-repair =
    { $count ->
        [one] इस आइटम को ठीक करें
       *[other] इन { $count } आइटम को ठीक करें
    }
finish-step-repair-detail = सिर्फ़ वही फिर आज़माएँ जो पूरा नहीं हुआ। जो चला वह वैसा ही रहता है।
finish-step-work = जाँचें कि आप यहाँ काम कर सकते हैं
finish-step-work-detail = देखें कि आपको जिन फ़ाइलों, फ़ॉन्ट और कामों की ज़रूरत है वे यहाँ हैं।
finish-step-kit = रिकवरी किट सहेजें
finish-step-kit-detail = इस स्थानांतरण का एक छोटा रिकॉर्ड अपने चुने फ़ोल्डर में रखें।
space-box-title = इस लैपटॉप पर सब कुछ रखने की जगह नहीं है
space-box-choice = { $count ->
        [one] जो { $count } फ़ाइल ({ $size }) आपने सबसे लंबे समय से इस्तेमाल नहीं की, उसे पुराने लैपटॉप पर छोड़ दें
       *[other] जो { $count } फ़ाइलें ({ $size }) आपने सबसे लंबे समय से इस्तेमाल नहीं कीं, उन्हें पुराने लैपटॉप पर छोड़ दें
    }
space-box-kept = कुछ भी मिटाया नहीं जाता। वे आपके पुराने लैपटॉप पर रहती हैं।
space-box-short = फिर भी { $size } की कमी है। कुछ फ़ोल्डरों से निशान हटाएँ, या उनके लिए कोई दूसरी जगह चुनें।
space-box-fits = उन्हें पुराने लैपटॉप पर छोड़ने से बाकी सब आ जाता है।
finish-handover-check-old = पुराने लैपटॉप को दोबारा जाँचें
finish-handover-check-armed = PCTwin पुराने लैपटॉप को जाँचने के लिए तैयार है। पुराने लैपटॉप पर PCTwin खोलें और दोनों लैपटॉप को वैसे ही जोड़ें जैसे ले जाने के लिए जोड़ते हैं। फिर दोबारा देखें चुनें।
finish-handover-look-again = दोबारा देखें
qs-where-title = इन्हें कहाँ रखें?
qs-where-body = { $count ->
        [one] { $count } फ़ाइल, { $size }। आपके चुनने तक कुछ भी सेव नहीं होता।
       *[other] { $count } फ़ाइलें, { $size }। आपके चुनने तक कुछ भी सेव नहीं होता।
    }
qs-where-folder = फ़ोल्डर: { $folder }
qs-where-details = और जानकारी
qs-where-path = पूरी जगह: { $path }
qs-where-save = यहाँ सेव करें
qs-where-choose = दूसरा फ़ोल्डर चुनें
qs-where-stop = रोकें

# 2.9 destination changed after approval
outcome-landed-moved = यह फ़ोल्डर स्थानांतरण के दौरान हटाया गया। आपकी फ़ाइलें इसी में हैं, यह अब जहाँ भी हो।
move-place-changed-title = आपकी मंज़ूरी के बाद एक फ़ोल्डर बदल गया
move-place-changed-body = जिस फ़ोल्डर में फ़ाइलें जानी थीं, वह आपकी मंज़ूरी के बाद हटाया या बदला गया। कुछ भी कॉपी नहीं हुआ। फ़ोल्डर देखें, फिर से शुरू करें।

# Quick Send stage 1
qs-refusal-not-paired = लैपटॉप अब जुड़े नहीं हैं। उन्हें फिर से जोड़ें।
qs-refusal-not-waiting = अभी इसके लिए कुछ भी इंतज़ार में नहीं है।
qs-refusal-failed = यह नहीं हुआ। फिर से कोशिश करें।
qs-refusal-note-empty = पहले भेजने के लिए कुछ लिखें।
qs-refusal-note-too-long = यह नोट बहुत लंबा है। 1,000 अक्षरों से कम रखें।
qs-refusal-note-too-many-lines = इस नोट में बहुत ज़्यादा पंक्तियाँ हैं। 20 तक रखें।
qs-refusal-note-hidden = इस नोट में ऐसा अक्षर है जिसे सुरक्षित रूप से नहीं दिखाया जा सकता। इसे फिर से लिखें।
qs-refusal-no-link = खोलने के लिए कोई लिंक नहीं है।
qs-refusal-not-found = यह अब यहाँ नहीं है।
qs-refusal-restored = यह लैपटॉप रिस्टोर या कॉपी किया गया था। दोनों लैपटॉप को कोड से फिर से जोड़ें।
qs-refusal-bad-name = 40 अक्षरों तक का नाम दें।
qs-refusal-store-unavailable = PCTwin इस लैपटॉप की चाबियों वाली सुरक्षित जगह तक नहीं पहुँच सका। साइन इन करने के बाद फिर से कोशिश करें।
qs-refusal-too-many-connections = आपके बहुत ज़्यादा कनेक्शन सहेजे गए हैं। पहले एक भूलें।
qs-refusal-folder-runs-at-sign-in = यह फ़ोल्डर साइन इन करते ही प्रोग्राम चलाता है। कोई दूसरा फ़ोल्डर चुनें।
qs-refusal-folder-shell-start-up = यह फ़ोल्डर टर्मिनल खुलते ही चीज़ें चलाता है। कोई दूसरा फ़ोल्डर चुनें।
qs-refusal-folder-holds-keys = इस फ़ोल्डर में चाबियाँ हैं। कोई दूसरा फ़ोल्डर चुनें।
qs-refusal-folder-system = यह सिस्टम का अपना फ़ोल्डर है। कोई दूसरा फ़ोल्डर चुनें।
qs-refusal-folder-not-plain = यह सामान्य फ़ोल्डर नहीं है। कोई दूसरा फ़ोल्डर चुनें।
qs-refusal-folder-not-the-same = यह वह फ़ोल्डर नहीं है जो आपने पहले चुना था। उसे फिर से चुनें।
qs-refusal-not-nearby = दूसरा लैपटॉप पास में नहीं है। वह बंद हो सकता है या किसी दूसरे नेटवर्क पर।
qs-refusal-over-cap = इंतज़ार में भेजे जाने वाले बहुत ज़्यादा जगह लेंगे। पहले एक रद्द करें।
qs-refusal-no-room = इस लैपटॉप पर पर्याप्त खाली जगह नहीं है।
qs-refusal-bad-days = 1 से 30 दिन चुनें।
qs-refusal-bad-recipients = हर व्यक्ति को एक बार चुनें, 20 तक।
qs-refusal-too-many-files = बहुत ज़्यादा फ़ाइलें हैं। कम चुनें।
qs-refusal-changed-while-copying = कॉपी होते समय एक फ़ाइल बदल गई। फिर से कोशिश करें।
qs-note-copy = कॉपी करें
qs-note-open = खोलें
qs-note-link-to = यह लिंक { $host } पर जाता है।
qs-note-sent = नोट पहुँच गया।
qs-remember-ask = इस कनेक्शन को याद रखें?
qs-remember-body = अगली बार आप बिना कोड के भेज सकेंगे। इसका मतलब है पहले वाला ही लैपटॉप, कोई व्यक्ति नहीं।
qs-remember-fallback = इस लैपटॉप पर चाबी आपके खाते की एक फ़ाइल में रखी जाती है।
qs-known-same-device = पहले वाला ही डिवाइस
qs-known-restored = यह लैपटॉप रिस्टोर या कॉपी किया गया था। फिर से जोड़ें।
qs-known-last-used = आखिरी बार इस्तेमाल: { $date }
qs-known-forget = भूलें
qs-known-forget-body = चाबी और सभी अनुमतियाँ हटा दी जाती हैं। पहले से भेजी गई फ़ाइलें जहाँ हैं वहीं रहती हैं।
qs-known-allow-folder = आगे के भेजने को एक फ़ोल्डर में आने दें
qs-known-allow-body = फ़ाइलें उसके अंदर PCTwin Quick Send नाम के फ़ोल्डर में जाती हैं। यह 30 दिन बाद अपने आप खत्म हो जाता है।
qs-known-wants-to-send = { $label } आपको यह भेजना चाहता है।
qs-known-delivered = { $label } से फ़ाइलें { $folder } में आ गईं।
qs-waiting-days =
    { $count ->
        [one] 1 दिन इंतज़ार
       *[other] { $count } दिन इंतज़ार
    }
qs-waiting-files =
    { $count ->
        [one] 1 फ़ाइल इंतज़ार में
       *[other] { $count } फ़ाइलें इंतज़ार में
    }
qs-waiting-newer = नया संस्करण मौजूद है। इसे फिर से देखें।
qs-waiting-on = यह लैपटॉप चालू रहने तक ही भेजता है।
qs-waiting-cancel = रद्द करें और कॉपी हटाएँ

# Quick Send stage 1 fixes
qs-known-cannot-tell = PCTwin यह नहीं बता सकता कि यह वही लैपटॉप है या नहीं।
qs-remember-replaces = क्या आपने इस लैपटॉप को पहले जोड़ा था? इसे चुनें, और पुराना कनेक्शन भुला दिया जाएगा।
qs-remember-replaces-suggested = यह फिर से { $label } हो सकता है। क्या इसे बदलें?
qs-remember-replace = बदलें
qs-remember-keep-both = नहीं, दोनों रखें

# 2.6 wiring
reason-open-in-app = पुराने लैपटॉप पर किसी ऐप ने यह फ़ाइल खोल रखी थी, इसलिए PCTwin ने इसे कॉपी नहीं किया (ऐप के लिखते समय बनी कॉपी खराब हो सकती है)। पुराने लैपटॉप पर वह ऐप बंद करें, फिर एक नया स्थानांतरण शुरू करें जिसमें यह फ़ाइल हो।
report-open-in = { $app } में खुली है
fullmove-close-files = ये फ़ाइलें इस लैपटॉप पर किसी ऐप में खुली हैं, इसलिए PCTwin अभी इन्हें सुरक्षित रूप से कॉपी नहीं कर सकता। इन्हें इस्तेमाल करने वाला ऐप बंद करें, फिर फिर से कोशिश करें चुनें। अगर आप इनके बिना आगे बढ़ते हैं, तो ये इसी लैपटॉप पर रहेंगी और रिपोर्ट में इनकी सूची होगी।
choose-online-only =
    { NUMBER($count) ->
        [one] { $provider } में सिर्फ़ ऑनलाइन रखी फ़ाइल भी लाएँ (1 फ़ाइल, { $size })। यह पहले इस लैपटॉप पर डाउनलोड होगी, फिर वापस सिर्फ़ ऑनलाइन कर दी जाएगी।
       *[other] { $provider } में सिर्फ़ ऑनलाइन रखी फ़ाइलें भी लाएँ ({ $count } फ़ाइलें, { $size })। ये पहले इस लैपटॉप पर डाउनलोड होंगी, फिर वापस सिर्फ़ ऑनलाइन कर दी जाएँगी।
    }
choose-online-only-unnamed = इस लैपटॉप की एक ऑनलाइन स्टोरेज सेवा
choose-online-only-metered = ऐसे कनेक्शन पर भी जो डेटा के पैसे ले सकता है (फ़ोन का हॉटस्पॉट)
choose-online-only-not-here = अगर आपकी कुछ फ़ाइलें सिर्फ़ ऑनलाइन रखी हैं, तो वे आपकी ऑनलाइन स्टोरेज में ही रहती हैं। PCTwin अभी इस लैपटॉप से उनकी कॉपी नहीं ला सकता।
kept-online-copied =
    { NUMBER($count) ->
        [one] सिर्फ़ ऑनलाइन रखी 1 फ़ाइल डाउनलोड करके कॉपी की गई।
       *[other] सिर्फ़ ऑनलाइन रखी { $count } फ़ाइलें डाउनलोड करके कॉपी की गईं।
    }
kept-online-safe =
    { NUMBER($count) ->
        [one] सिर्फ़ ऑनलाइन रखी 1 फ़ाइल कॉपी नहीं हुई। वह अब भी ऑनलाइन रखी है: नए लैपटॉप पर उसे रखने वाले ऐप से खोलें।
       *[other] सिर्फ़ ऑनलाइन रखी { $count } फ़ाइलें कॉपी नहीं हुईं। वे अब भी ऑनलाइन रखी हैं: नए लैपटॉप पर उन्हें रखने वाले ऐप से खोलें।
    }
kept-online-still-here =
    { NUMBER($count) ->
        [one] डाउनलोड की गई 1 फ़ाइल अभी भी इस लैपटॉप पर { $size } जगह ले रही है, क्योंकि उसके क्लाउड ऐप ने उसे फिर से सिर्फ़ ऑनलाइन नहीं किया है। जगह खाली करने के लिए उस पर राइट-क्लिक करें और जगह खाली करें चुनें।
       *[other] डाउनलोड की गई { $count } फ़ाइलें अभी भी इस लैपटॉप पर { $size } जगह ले रही हैं, क्योंकि उनके क्लाउड ऐप ने उन्हें फिर से सिर्फ़ ऑनलाइन नहीं किया है। जगह खाली करने के लिए हर एक पर राइट-क्लिक करें और जगह खाली करें चुनें।
    }
kept-online-paused-metered = डाउनलोड रुक गया क्योंकि यह कनेक्शन डेटा के पैसे ले सकता है। बाकी फ़ाइलें कॉपी नहीं हुईं और अब भी ऑनलाइन रखी हैं।
kept-online-paused-space = डाउनलोड रुक गया क्योंकि इस लैपटॉप में जगह खत्म हो रही है। बाकी फ़ाइलें कॉपी नहीं हुईं और अब भी ऑनलाइन रखी हैं।
plan-own-folder = { $laptop } की चीज़ें उनके अपने फ़ोल्डर में रखें
plan-own-folder-help = इस लैपटॉप पर पहले से किसी दूसरे पुराने लैपटॉप की चीज़ें हैं। हर लैपटॉप का अपना फ़ोल्डर उन्हें अलग रखता है।
plan-own-folder-name = { $laptop } से
plan-own-folder-name-unnamed = दूसरे लैपटॉप से
plan-downloads =
    { NUMBER($count) ->
        [one] इस लैपटॉप पर इंस्टॉल करने के लिए 1 ऐप इंटरनेट से डाउनलोड होगा।
       *[other] इस लैपटॉप पर इंस्टॉल करने के लिए { $count } ऐप इंटरनेट से डाउनलोड होंगे।
    }
plan-wait-free-wifi = डाउनलोड करने से पहले मुफ़्त Wi-Fi का इंतज़ार करें
plan-wait-free-wifi-help = अगर ऐप डाउनलोड होते समय कनेक्शन डेटा के पैसे ले सकता है, तो PCTwin मुफ़्त Wi-Fi के लिए 20 मिनट तक इंतज़ार करता है। जो ऐप डाउनलोड नहीं हो पाए, वे बाद में इंस्टॉल करने के लिए सूची में दिखेंगे।
plan-reuse-installers =
    { NUMBER($count) ->
        [one] 1 ऐप के लिए पुराने लैपटॉप पर मिला इंस्टॉलर इस्तेमाल करें, उसे डाउनलोड करने के बजाय
       *[other] { $count } ऐप के लिए पुराने लैपटॉप पर मिले इंस्टॉलर इस्तेमाल करें, उन्हें डाउनलोड करने के बजाय
    }
plan-reuse-installers-help = हर एक की पहले जाँच होती है, ठीक डाउनलोड की तरह: उस पर ऐप के अपने निर्माता के हस्ताक्षर होने चाहिए।
fullmove-close-files-title = इन फ़ाइलों को इस्तेमाल करने वाले ऐप बंद करें
choose-online-only-title = सिर्फ़ ऑनलाइन रखी फ़ाइलें
kept-online-title = सिर्फ़ ऑनलाइन रखी फ़ाइलें
kept-online-downloaded = { $size } डाउनलोड हुआ
plan-own-folder-called = फ़ोल्डर का नाम { $folder } है।
accept-own-folder = दूसरे लैपटॉप की चीज़ें उनके अपने फ़ोल्डर { $folder } में जाती हैं। आप विवरण में इसे बंद कर सकते हैं।
plan-downloads-title = इंटरनेट से ऐप
plan-connection-free = इस लैपटॉप का कनेक्शन अभी डेटा के पैसे नहीं लेता।
plan-connection-may-charge = इस लैपटॉप का कनेक्शन अभी डेटा के पैसे ले सकता है, जैसे फ़ोन का हॉटस्पॉट।
plan-connection-not-known = PCTwin यह नहीं बता सकता कि इस लैपटॉप का कनेक्शन डेटा के पैसे लेता है या नहीं।
fullmove-rebuilding-wait-wifi = अगर यह कनेक्शन डेटा के पैसे ले सकता है, तो PCTwin ऐप डाउनलोड करने से पहले मुफ़्त Wi-Fi के लिए 20 मिनट तक इंतज़ार करता है। जल्दी आगे बढ़ने के लिए मुफ़्त Wi-Fi से जुड़ें।

# Quick Send stage 1 screens
qs-note-card-title = नोट या लिंक भेजें
qs-note-card-body = कुछ पंक्तियाँ, या एक वेब लिंक।
qs-home-known = याद रखे गए कनेक्शन
qs-home-waiting = प्रतीक्षा में भेजे गए
qs-note-title = नोट या लिंक भेजें
qs-note-intro = कुछ पंक्तियाँ लिखें, या एक वेब लिंक चिपकाएँ। सिर्फ़ यहाँ लिखा हुआ भेजा जाता है।
qs-note-label = नोट या लिंक
qs-note-hint = ज़्यादा से ज़्यादा 20 पंक्तियाँ और 1,000 अक्षर।
qs-note-never-opens = आपका भेजा लिंक तभी खुलता है जब पाने वाला खोलें चुने।
qs-note-sending-title = नोट भेजा जा रहा है
qs-note-sending-body = कृपया रुकें, पाने वाला लैपटॉप इसे ले रहा है।
qs-note-sent-title = नोट पहुँच गया
qs-note-sent-body = पाने वाले लैपटॉप पर पहुँच गया। PCTwin नहीं बता सकता कि इसे पढ़ा गया या नहीं।
qs-note-received-title = एक नोट आया
qs-note-received-link-title = एक लिंक आया
qs-note-from = { $label } से।
qs-note-from-code = उस लैपटॉप से जिसे आपने कोड से जोड़ा।
qs-note-link-safety = यह आपके ब्राउज़र में तभी खुलता है जब आप खोलें चुनें।
qs-note-copied = कॉपी हो गया।
qs-note-copy-failed = PCTwin इसे कॉपी नहीं कर सका। टेक्स्ट चुनकर खुद कॉपी करें।
qs-remember-name = इस कनेक्शन का नाम
qs-remember-name-hint = 40 अक्षरों तक, ताकि बाद में ढूँढ सकें।
qs-remember-save = याद रखें
qs-remember-done = { $label } नाम से याद रखा गया। इसकी फ़ाइलें अब भी पहले आपसे पूछती हैं।
qs-known-title = याद रखे गए कनेक्शन
qs-known-intro = लैपटॉप जिन्हें आप बिना कोड के भेज सकते हैं। हर एक का मतलब पहले वाला ही लैपटॉप है, कोई व्यक्ति नहीं।
qs-known-empty = अभी कोई कनेक्शन याद नहीं रखा गया है। कोड से भेजने या पाने के बाद PCTwin पूछता है कि क्या वह लैपटॉप याद रखना है।
qs-known-expires = अगर तब तक इस्तेमाल न हुआ तो { $date } को अपने आप भूल जाएगा।
qs-known-rename = नाम बदलें
qs-known-rename-label = नया नाम
qs-known-rename-save = नाम सेव करें
qs-known-forget-title = { $label } को भूल जाएँ?
qs-known-forget-confirm = भूल जाएँ
qs-known-folder-on = इसकी फ़ाइलें { $folder } में, PCTwin Quick Send नाम के फ़ोल्डर में जाती हैं।
qs-known-folder-ends = यह { $date } को खत्म होगा।
qs-known-folder-ended = यह { $date } को खत्म हो गया। इसकी फ़ाइलें फिर से पहले आपसे पूछती हैं।
qs-known-renew = 30 दिन के लिए बढ़ाएँ
qs-known-next-title = अगली बार का फ़ोल्डर
qs-known-next-on = जब यह आपको फ़ाइलें भेजता है तो { $folder } पहले से चुना होता है। आप हर बार इसे बदल सकते हैं।
qs-known-next-none = तय नहीं है। जब यह फ़ाइलें भेजता है तो डाउनलोड पहले से चुना होता है, या आप हर बार दूसरा फ़ोल्डर चुनते हैं।
qs-known-next-use = आखिरी फ़ोल्डर इस्तेमाल करें
qs-known-next-clear = इसे इस्तेमाल न करें
qs-known-in-more = और { $count } अन्य
qs-known-in-next = अगली बार { $label } के लिए यही फ़ोल्डर इस्तेमाल करें
qs-known-in-decline = मना करें
qs-known-delivered-counts = { $total } में से { $arrived } कॉपी और जाँचे गए।
qs-send-to = { $name } को भेजें
qs-to-title = इन्हें किसे मिलना चाहिए?
qs-to-intro = याद रखे गए लैपटॉप चुनें, या किसी लैपटॉप को कोड से जोड़ें।
qs-to-known = याद रखे गए लैपटॉप
qs-to-code = इसके बजाय कोड इस्तेमाल करें
qs-to-when = कब
qs-to-now = अभी
qs-to-now-hint = पाने वाला लैपटॉप चालू और पास होना चाहिए।
qs-to-later = जब यह पास हो
qs-to-several = हर लैपटॉप को ये तब मिलती हैं जब वह पास हो।
qs-to-days = कितनी देर प्रतीक्षा करें
qs-to-each-asks = पाने वाले लैपटॉप से पूछा जाता है कि कहाँ सेव करें, जब तक उसने किसी फ़ोल्डर में भेजने की अनुमति न दी हो।
qs-to-approve-hint = आप इन फ़ाइलों को अभी जैसी हैं, इन लैपटॉप और इस समय के लिए मंज़ूरी देते हैं। PCTwin भेजने के लिए फ़ाइलों की एक कॉपी रखता है। हर लैपटॉप से पूछा जाता है कि कहाँ सेव करें, जब तक उसने अनुमति न दी हो।
qs-to-send-now = अभी भेजें
qs-to-approve = इस डिलीवरी को मंज़ूरी दें
qs-waiting-title = प्रतीक्षा में भेजे गए
qs-waiting-intro = फ़ाइलें जिनकी कॉपी PCTwin रखता है, ताकि हर लैपटॉप के पास होने पर भेज सके।
qs-waiting-empty = कुछ भी प्रतीक्षा में नहीं है।
qs-waiting-approved = { $date } को मंज़ूर
qs-waiting-until = { $date } तक प्रतीक्षा
qs-waiting-ended = { $date } को खत्म हुआ। अब और कुछ नहीं भेजा जाएगा।
qs-waiting-to-unknown = भुलाया गया कनेक्शन
qs-waiting-state-waiting = प्रतीक्षा में
qs-waiting-state-delivered = { $total } में से { $arrived } कॉपी और जाँचे गए
qs-waiting-state-needs = कार्रवाई ज़रूरी: { $count }
qs-waiting-state-declined = { $date } को मना किया
qs-waiting-state-failed = कॉपी नहीं हुए: { $count }
qs-waiting-renew-label = अब से कितनी देर प्रतीक्षा करें
qs-waiting-renew = बढ़ाएँ
qs-waiting-cancel-title = यह प्रतीक्षा वाला भेजना रद्द करें?
qs-waiting-cancel-body = PCTwin अपनी कॉपी मिटा देता है, और कुछ और नहीं भेजा जाता। जिन लैपटॉप के पास फ़ाइलें हैं, वे उन्हें रखते हैं। आपकी अपनी फ़ाइलें जैसी हैं वैसी रहती हैं।
qs-waiting-keep = रहने दें

# Quick Send more
qs-session-add-files = फ़ाइलें जोड़ें
qs-session-send-to = { $name } को भेजें
qs-session-send-back = फ़ाइलें वापस भेजें
qs-session-end = खत्म करें
qs-session-ended = यह भेजना खत्म हो गया। अब कुछ और नहीं आएगा।
qs-sends-waiting = अपनी बारी का इंतज़ार
qs-sends-sending = भेजा जा रहा है
qs-sends-paused = रुका हुआ
qs-send-pause = रोकें
qs-send-resume = जारी रखें
qs-send-cancel = रद्द करें

# Open sends: Add files, Send to, several laptops at once, questions that wait, a live file being copied
qs-session-ended-name = { $name }: यह भेजना खत्म हो गया। अब कुछ और नहीं आएगा।
qs-sends-action-name = { $what }: { $name }
qs-sends-title = आपके भेजे जा रहे काम
qs-sends-intro = PCTwin एक साथ ज़्यादा से ज़्यादा तीन लैपटॉप को भेजता है, और एक ही लैपटॉप को एक के बाद एक। बाकी अपनी बारी का इंतज़ार करते हैं।
qs-sends-empty = अभी कुछ नहीं भेजा जा रहा है।
qs-sends-ended-title = पूरे हुए भेजे गए काम
qs-sends-stopped = इसे अंत से पहले रोक दिया गया।
qs-sends-result-whole = कॉपी और जाँचे गए
qs-sends-result-partial = कुछ कॉपी नहीं हुए
qs-sends-not-sent = { $name } को नहीं भेजा गया।
qs-sends-failed = PCTwin उस भेजने को बदल नहीं सका। फिर कोशिश करें।
qs-session-offer-waiting = { $name } आपको और फ़ाइलें भेज रहा है।
qs-session-offer-show = सेव करने की जगह चुनें
qs-to-now-several-hint = हर लैपटॉप चालू और पास होना चाहिए। PCTwin एक साथ ज़्यादा से ज़्यादा तीन को भेजता है; बाकी अपनी बारी का इंतज़ार करते हैं।
qs-asking-files = { $label } आपको फ़ाइलें भेजना चाहता है।
qs-asking-note = { $label } ने आपको एक नोट भेजा।
qs-asking-later = जब आप यह पेज छोड़ेंगे, PCTwin आपसे इसके बारे में पूछेगा।
qs-asking-show = सेव करने की जगह चुनें
qs-asking-show-note = नोट दिखाएँ
progress-current-open-in = जब तक PCTwin { $name } कॉपी करता है, { $app } उसमें बदलाव सेव नहीं कर सकता। अगर { $app } कहे कि फ़ाइल इस्तेमाल में है, तो कॉपी पूरी होने का इंतज़ार करें।

# 2.6 waiting for free Wi-Fi
fullmove-rebuilding-waiting-wifi = { $minutes ->
        [one] ऐप डाउनलोड करने से पहले मुफ़्त Wi-Fi का इंतज़ार: ज़्यादा से ज़्यादा 1 मिनट और
       *[other] ऐप डाउनलोड करने से पहले मुफ़्त Wi-Fi का इंतज़ार: ज़्यादा से ज़्यादा { $minutes } मिनट और
    }
