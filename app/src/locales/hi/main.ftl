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
home-restore-body = उसे वापस लाएँ
home-rescue-body = ज़रूरी चीज़ें बचाएँ
home-history = मेरा इतिहास
home-history-body = आपके स्थानांतरण और उनकी रिपोर्ट
home-update-title = नया संस्करण उपलब्ध है।
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

connect-prep-title = दोनों लैपटॉप पास-पास रखें।
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
peer-title = अपने दूसरे लैपटॉप से मिलें।
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
check-title = आइए कनेक्शन की जाँच करें।
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
# Added 23 October 2026: the short checklist both laptops show before the move.
check-before-title = स्थानांतरण शुरू होने से पहले
check-before-lids-title = दोनों लैपटॉप के ढक्कन खुले रखें
check-before-lids-body = ढक्कन बंद करने से लैपटॉप स्लीप में जा सकता है, जिससे स्थानांतरण रुक जाता है।
check-before-near-title = Wi-Fi राउटर के पास रहें
check-before-near-body = जब तक आपकी चीज़ें जा रही हैं, दोनों लैपटॉप उसके पास रखें।

## C09 Waiting for your other laptop

reconnect-title = आपके दूसरे लैपटॉप का इंतज़ार है।
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

ended-title = कनेक्शन ख़त्म हो गया है।
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
pair-heads-up-title = आपका लैपटॉप आपसे एक सवाल पूछ सकता है।
pair-heads-up-local-network = आपका Mac पूछेगा कि क्या PCTwin आपके लोकल नेटवर्क पर डिवाइस ढूँढ सकता है। “अनुमति दें” चुनें, ताकि आपके दोनों लैपटॉप एक-दूसरे को ढूँढ सकें।
pair-heads-up-firewall = Windows पूछ सकता है कि क्या PCTwin कनेक्शन स्वीकार कर सकता है। “अनुमति दें” चुनें, ताकि आपका नया लैपटॉप इस लैपटॉप तक पहुँच सके।
pair-heads-up-continue = जारी रखें

## C03 Old laptop: show the code

pair-old-eyebrow = पुराना लैपटॉप
pair-old-title = आइए आपके लैपटॉप जोड़ें।
pair-old-body = अपने नए लैपटॉप पर नीचे दिया गया कोड डालें।
pair-old-next = इसके बाद, दोनों लैपटॉप पर दिखने वाला एक नंबर मिलाना होगा।
pair-old-network = दोनों लैपटॉप एक ही Wi-Fi पर रखें।
pair-old-waiting = आपके नए लैपटॉप का इंतज़ार है…
pair-old-starting = कोड तैयार हो रहा है…
pair-old-label-title = अभी के लिए इस लैपटॉप का नाम
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
pair-old-wait-title = नया कोड आ रहा है।
pair-old-wait-body =
    { $seconds ->
        [one] वह कोशिश काम नहीं आई। नया कोड { $seconds } सेकंड में दिखेगा।
       *[other] वह कोशिश काम नहीं आई। नया कोड { $seconds } सेकंड में दिखेगा।
    }
pair-old-locked-title = इस नेटवर्क पर किसी ने गलत कोड आज़माए।
pair-old-locked-body = पाँच गलत कोड के बाद, इस लैपटॉप को सुरक्षित रखने के लिए जोड़ना बंद कर दिया गया। कुछ भी शेयर नहीं हुआ। अगर यह आप थे, तो नया कोड पाने के लिए “फिर से शुरू करें” चुनें।
pair-old-locked-hotspot = अगर आपको नहीं पता कि यह कौन था, तो दोनों लैपटॉप को अपने फ़ोन के हॉटस्पॉट से कनेक्ट करें, फिर “फिर से शुरू करें” चुनें।
pair-old-start-again = फिर से शुरू करें
pair-old-timed-out = समय पर कोई नंबर नहीं चुना गया, इसलिए यह नया कोड है।
pair-old-declined = नंबर मेल नहीं खाए, इसलिए कुछ भी शेयर नहीं हुआ। यह नया कोड है।
pair-old-left = नंबर चुने जाने से पहले नए लैपटॉप ने जोड़ना बंद कर दिया, इसलिए कुछ भी शेयर नहीं हुआ। यह नया कोड है।
pair-network-title = अपना Wi-Fi चुनें।
pair-network-hint = वह नेटवर्क चुनें जिस पर आपका नया लैपटॉप है।

## C05 Old laptop: match the number

pair-choose-eyebrow = कनेक्शन की पुष्टि करें
pair-choose-title = मिलता हुआ नंबर चुनें।
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
pair-new-search-failed-title = आपका पुराना लैपटॉप ढूँढा नहीं जा सका।
pair-new-list-title = इनमें से आपका पुराना लैपटॉप कौन-सा है?
pair-new-list-body = वह नाम चुनें जो आपका पुराना लैपटॉप अपनी स्क्रीन पर दिखाता है।
pair-new-same-label = आस-पास के एक और लैपटॉप का भी यही नाम है। पुराने लैपटॉप की स्क्रीन देखें।
pair-new-needs-update = इस लैपटॉप पर PCTwin का पुराना संस्करण है। उस पर PCTwin अपडेट करें, फिर दोबारा खोजें।
pair-new-none-title = कोई पुराना लैपटॉप नहीं मिला।
pair-new-none-body = जाँचें कि आपके पुराने लैपटॉप पर PCTwin खुला है और कोड दिखा रहा है, और दोनों लैपटॉप एक ही Wi-Fi पर हैं। फिर से खोजें।
pair-new-search-again = फिर से खोजें
pair-old-same-role = पास का एक और लैपटॉप भी पुराने लैपटॉप के रूप में सेट है। दोनों में से एक नया लैपटॉप होना चाहिए।
pair-old-switch = यह नया लैपटॉप है
pair-new-none-same-role = क्या दूसरे लैपटॉप ने भी नया लैपटॉप चुना है? तो उस पर पुराना लैपटॉप चुनें, या इसे बदलें।
pair-new-switch = यह पुराना लैपटॉप है
pair-new-code-title = कोड डालें।
pair-new-code-body = { $label } पर दिखाया गया कोड लिखें।
pair-new-code-label = छह अंकों का कोड
pair-new-code-hint = इसे अपने पुराने लैपटॉप की स्क्रीन से पढ़ें। यह हर मिनट बदलता है।
pair-new-connect = कनेक्ट करें
pair-new-connecting = कनेक्ट हो रहा है…
pair-new-other-laptop = कोई दूसरा लैपटॉप चुनें
pair-new-number-eyebrow = कनेक्शन की पुष्टि करें
pair-new-number-title = अपने पुराने लैपटॉप पर यह नंबर चुनें।
pair-new-number-tile = आपका नंबर
pair-new-number-label = आपका नंबर { $number } है
pair-new-number-body = आपका पुराना लैपटॉप तीन नंबर दिखाता है। वहाँ यही नंबर चुनें।
pair-new-number-waiting = आपके पुराने लैपटॉप का इंतज़ार है…

## Paired (both laptops)

pair-done-eyebrow = सब तैयार
pair-done-title = आपके लैपटॉप जोड़ दिए गए हैं।
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
people-title = लोगों को उनके अकाउंट से मिलाएं।
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
before-title = इस लैपटॉप को छोड़ने से पहले।
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
help-title = इस काम में मदद पाएं।
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
help-home-title = आपके सामने वाले काम के लिए मदद।
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
scan-title = आपका जाना-पहचाना सेटअप ढूंढा जा रहा है।
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
overview-title = आपकी चुनी हर चीज़, एक नज़र में।
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
files-title = जो आपके लिए ज़रूरी है, उसे साथ रखें।
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
dest-title = हर फ़ाइल को उसकी जगह दें।
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
plan-title = अपना मनचाहा स्थानांतरण मंज़ूर करें।
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
plan-version = योजना वर्शन { $revision }
plan-details = और जानकारी
plan-preview-title = शुरू करने से पहले
plan-preview-fits = इस लैपटॉप में पर्याप्त जगह खाली है।
# Added 23 October 2026: what moves first, from the move's own order. $places is a list of folder names.
plan-preview-first = हम पहले ये भेजेंगे, ताकि आपका नया लैपटॉप जल्दी इस्तेमाल के लिए तैयार हो: { $places }।
plan-preview-short = इस लैपटॉप को { $size } और खाली जगह चाहिए। शुरू करने से पहले कुछ जगह खाली करें, वरना ड्राइव भरने पर स्थानांतरण रुक जाएगा।
plan-preview-space-unknown = PCTwin यह नहीं पढ़ सका कि इस लैपटॉप में कितनी जगह खाली है।
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
progress-title = आपका नया लैपटॉप तैयार हो रहा है।
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
progress-compact = छोटा करें
progress-expand = पूरा स्थानांतरण देखें
progress-stop = स्थानांतरण बंद करें

## X02 Move progress, expanded

progress-all-title = पूरा स्थानांतरण देखें।
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

mini-title = आपका स्थानांतरण, आस-पास।
mini-title-paused = स्थानांतरण रुका हुआ है।
mini-what = आपकी चुनी फ़ाइलें भेजी जा रही हैं
mini-progress =
    .aria-label = स्थानांतरण की प्रगति
mini-pause = रोकें
mini-resume = फिर से शुरू करें
mini-open = स्थानांतरण खोलें
# Used when lane E reuses this panel for a Quick Send.
mini-title-send = आपकी फ़ाइलें रास्ते में हैं।
mini-what-send = चुनी गई फ़ाइलें भेजी जा रही हैं
mini-open-send = भेजना खोलें
mini-progress-send =
    .aria-label = भेजने की प्रगति

## X05 Paused at a safe point

pause-title = एक सुरक्षित पड़ाव पर रोका गया।
pause-title-finishing = सुरक्षित पड़ाव पर रोका जा रहा है…
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
pause-resume = सहेजा गया स्थानांतरण फिर से शुरू करें

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
outcome-title = आपके अगले लैपटॉप पर आपका स्वागत है।
outcome-body = आपने जो कुछ मंज़ूर किया था, उसके साथ क्या हुआ, यहाँ देखें।
outcome-files-title = चुनी गई फ़ाइलें
outcome-status-partial = कुछ हिस्सा पूरा
# Added 23 October 2026: the after-move check on R01, counted the way the scan counts.
outcome-kinds-title = स्थानांतरण के बाद जाँचा गया
outcome-kind-photos = फ़ोटो
outcome-kind-videos = वीडियो
outcome-kind-music = संगीत
outcome-kind-documents = दस्तावेज़
outcome-kind-count = { $planned } में से { $copied } कॉपी और जाँचे गए
outcome-next-title = आगे क्या करें
outcome-old-safe = जो पूरा नहीं हुआ, वह यहाँ सूची में रहता है। आपके पुराने लैपटॉप से कुछ भी नहीं हटाया गया।
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
pickup-title = सुरक्षित रूप से आगे बढ़ें।
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
pickup-done = सब व्यवस्थित है। आगे बढ़ने के लिए लैपटॉप दोबारा जोड़ें: जो यहाँ पहले से है वह पहचाना जाता है और दोबारा कॉपी नहीं होता।
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
ready-eyebrow = स्वागत है
ready-title = आपकी चीज़ें तैयार हैं।
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
move-wait-scanning-title = आपकी फ़ाइलें ढूँढी जा रही हैं।
move-wait-scanning-body = आपका पुराना लैपटॉप आपके फ़ोल्डर देख रहा है। वह सिर्फ़ उन्हें पढ़ता है: कुछ भी खोला, बदला या हटाया नहीं जाता।
move-found = अब तक मिला: आपकी चीज़ों में से { $size }।
move-wait-approval-title = नए लैपटॉप का इंतज़ार है।
move-wait-approval-body = आपकी फ़ाइलें तैयार हैं। नए लैपटॉप पर योजना देखें और शुरू करने के लिए उसे मंज़ूर करें।
move-wait-list-title = पुराने लैपटॉप का इंतज़ार है।
move-wait-list-body = आपका पुराना लैपटॉप आपकी फ़ाइलें ढूँढ रहा है। पूरा होने पर योजना यहाँ मंज़ूरी के लिए दिखेगी। इसमें कुछ मिनट लग सकते हैं।
move-wait-starting-title = स्थानांतरण शुरू हो रहा है।
move-wait-starting-body = योजना मंज़ूर हो गई है। कॉपी करना कुछ ही पल में शुरू होगा।
move-wait-admin-title = एडमिनिस्ट्रेटर का इंतज़ार है।
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
fda-title = आपका Mac कुछ फ़ोल्डर निजी रख रहा है।
fda-body =
    { NUMBER($count) ->
        [one] Mac की निजता सेटिंग की वजह से आपकी { $count } जगह देखी नहीं जा सकी। PCTwin को अनुमति दें, फिर दोबारा जाँचें, ताकि आपकी कोई चीज़ न छूटे।
       *[other] Mac की निजता सेटिंग की वजह से आपकी { $count } जगहें देखी नहीं जा सकीं। PCTwin को अनुमति दें, फिर दोबारा जाँचें, ताकि आपकी कोई चीज़ न छूटे।
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
move-lost-title = आपके दूसरे लैपटॉप से कनेक्शन टूट गया।
move-lost-body = लैपटॉप एक-दूसरे तक नहीं पहुँच पाए, इसलिए स्थानांतरण आगे नहीं बढ़ सका। देखें कि दोनों चालू हैं, जागे हुए हैं और पास-पास हैं।
move-not-approved-title = स्थानांतरण शुरू नहीं हुआ।
move-not-approved-old = नए लैपटॉप पर योजना मंज़ूर नहीं हुई, इसलिए कुछ भी कॉपी नहीं हुआ।
move-not-approved-new = आपने योजना मंज़ूर नहीं की, इसलिए कुछ भी कॉपी नहीं हुआ।
move-failed-title = कुछ गड़बड़ हो गई।
move-failed-body = स्थानांतरण आगे नहीं बढ़ सका। अगर ऐसा फिर हो, तो नीचे ज़्यादा जानकारी में बताया गया है कि क्या गड़बड़ हुई।
move-peer-system-windows = Windows
move-peer-system-mac = Mac
move-peer-system-linux = Linux
move-peer-system-other = कोई दूसरा सिस्टम
move-update-here-title = इस लैपटॉप पर PCTwin अपडेट करें
move-update-here-body = इस लैपटॉप पर PCTwin दूसरे लैपटॉप से पुराना है, इसलिए दोनों मिलकर फ़ाइलें नहीं ले जा सकते। यहाँ PCTwin अपडेट करें, फिर से शुरू करें। कुछ भी कॉपी नहीं हुआ।
move-update-there-title = दूसरे लैपटॉप पर PCTwin अपडेट करें
move-update-there-body = दूसरे लैपटॉप पर PCTwin इस लैपटॉप से पुराना है, इसलिए दोनों मिलकर फ़ाइलें नहीं ले जा सकते। वहाँ PCTwin अपडेट करें, फिर से शुरू करें। कुछ भी कॉपी नहीं हुआ।
move-refused-title = स्थानांतरण शुरू नहीं हो सका।
move-refused-not-paired = आपके लैपटॉप अब जुड़े नहीं हैं। स्थानांतरण शुरू करने के लिए उन्हें फिर से जोड़ें।
move-refused-busy = इस लैपटॉप पर पहले से एक स्थानांतरण चल रहा है।
move-refused-not-moving = इस लैपटॉप पर कोई स्थानांतरण नहीं चल रहा है।
move-refused-plan-changed = मंज़ूरी से पहले योजना बदल गई। नई योजना देखने के लिए लैपटॉप फिर से जोड़ें।
move-refused-failed = इस लैपटॉप के फ़ोल्डर स्थानांतरण के लिए तैयार नहीं हो सके। फिर से कोशिश करने के लिए लैपटॉप दोबारा जोड़ें।
drive-problem-title = स्थानांतरण को आपका ध्यान चाहिए।
drive-problem-body = PCTwin ने कई बार जाँचा, और { $place } में रखी कॉपियाँ मूल फ़ाइलों से मेल नहीं खाईं। ड्राइव ख़राब, भरी हुई या निकली हुई हो सकती है, इसलिए वहाँ कॉपी करना रुका है।
drive-problem-kept-title = पहले से कॉपी और जाँचा गया
drive-problem-waiting-title = { $place } में अभी कॉपी होना बाकी
drive-problem-badge = रुका हुआ
drive-problem-fix = देखें कि ड्राइव लगी है और उसमें जगह है, फिर आगे बढ़ें। इससे बचने के लिए आपकी चुनी कोई चीज़ छोड़ी नहीं जाती। आप स्थानांतरण रोक भी सकते हैं।
drive-problem-carry-on = जारी रखें

## R02 Your move, item by item (detailed report)

report-title = आपका स्थानांतरण, एक-एक आइटम।
report-body = हर आइटम के साथ क्या हुआ, क्यों हुआ, और वह कहाँ से आया और कहाँ गया।
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
reason-sign-in = यह अपनी जगह पर है, लेकिन आपको साइन इन करना होगा। इसे खोलें और साइन इन करें।
reason-activation = ऐप अपनी जगह पर है, लेकिन नए लैपटॉप पर इसे इसका लाइसेंस चाहिए। इसे खोलें और अपने लाइसेंस या अकाउंट से इसे चालू करें।
reason-permission = नए लैपटॉप को पहले अनुमति देनी होगी। आइटम खोलें और पूछे जाने पर “अनुमति दें” चुनें।
reason-online-only = यह फ़ाइल सिर्फ़ ऑनलाइन रखी है, इसलिए कॉपी करने के लिए पुराने लैपटॉप पर नहीं थी। इसे नए लैपटॉप पर उसी ऐप से डाउनलोड करें जिसमें यह ऑनलाइन रखी है।
reason-manual-install = यह ऐप इसके बनाने वाले की वेबसाइट से ही आना चाहिए। इसे वहीं से डाउनलोड और इंस्टॉल करें।
reason-restart = पूरा करने के लिए नए लैपटॉप को रीस्टार्ट करना होगा। तैयार होने पर इसे रीस्टार्ट करें।
reason-finish-by-hand = PCTwin यह कदम आपके लिए नहीं कर सकता। इस रिपोर्ट में इसके लिए दिखाए गए कदम अपनाएं।
reason-parts-missing = इस फ़ोल्डर की कुछ फ़ाइलें नहीं पहुँचीं, इसलिए नए लैपटॉप पर यह पूरा नहीं है। वे इस रिपोर्ट में कारण के साथ दी गई हैं। उन्हें फिर से भेजें, या फ़ोल्डर को किसी और तरीके से कॉपी करें।

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
mini-title-send-paused = भेजना रुका हुआ है।
cancel-ack-hint = बंद करने का बटन चालू करने के लिए ऊपर वाले बॉक्स पर टिक करें।
progress-state-placing = जगह पर रखा जा रहा है
progress-state-existing = पहले से मौजूद
outcome-stopped-title = आपका स्थानांतरण बंद कर दिया गया।
outcome-stopped-body = स्थानांतरण आपने बंद किया। उससे पहले जो कॉपी और जाँचा गया था, वह रखा गया है। बाकी कॉपी नहीं हुआ।
outcome-broke-title = आपका स्थानांतरण समय से पहले खत्म हो गया।
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
qs-pick-too-many = ये एक बार भेजने में जा सकने वाली फ़ाइलों से ज़्यादा हैं। कुछ फ़ोल्डरों से टिक हटाएँ, या इन्हें हिस्सों में भेजें।
qs-problem-lost-title = लैपटॉप के बीच कनेक्शन टूट गया।
qs-problem-failed-title = इस बार फ़ाइलें नहीं भेजी जा सकीं।
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

qs-home-title = आस-पास के किसी लैपटॉप पर फ़ाइलें भेजें।
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

qs-sending-title = आपकी फ़ाइलें रास्ते में हैं।
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
qs-compact = छोटा करें
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

history-title = आपके स्थानांतरण, इस लैपटॉप पर रखे हुए।
history-body = कोई रिपोर्ट फिर से खोलें, या सहेजा गया स्थानांतरण आगे बढ़ाएं।
history-row-move = { $from } → { $to }
history-status-finished = पूरा हुआ
history-status-steps-left =
    { NUMBER($count) ->
        [one] पूरा हुआ, आपके लिए { $count } चरण बाकी
       *[other] पूरा हुआ, आपके लिए { $count } चरण बाकी
    }
history-status-paused = रुका हुआ
history-status-stopped = सुरक्षित रूप से बंद
history-open-report = रिपोर्ट
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
qs-pair-new-search-failed-title = भेजने वाले लैपटॉप को खोजा नहीं जा सका।
qs-pair-new-list-title = कौन-सा लैपटॉप भेज रहा है?
qs-pair-new-list-body = वह नाम चुनें जो भेजने वाला लैपटॉप अपनी स्क्रीन पर दिखाता है।
qs-pair-new-same-label = पास के किसी दूसरे लैपटॉप का भी यही नाम है। भेजने वाले लैपटॉप की स्क्रीन देखें।
qs-pair-new-none-title = भेजने वाला कोई लैपटॉप नहीं मिला।
qs-pair-new-none-body = जाँचें कि भेजने वाले लैपटॉप पर PCTwin खुला है और कोड दिखा रहा है, और दोनों लैपटॉप एक ही Wi-Fi पर हैं। फिर से खोजें।
qs-pair-new-code-hint = इसे { $label } की स्क्रीन से पढ़ें। यह हर मिनट बदलता है।
qs-pair-new-number-title = { $label } पर यह नंबर चुनें।
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
qs-done-sent-title = आपकी फ़ाइलें भेज दी गईं।
qs-done-here-title = आपकी फ़ाइलें आ गई हैं।
qs-done-partial-title = कुछ फ़ाइलें नहीं पहुँचीं।
qs-done-stopped-title = यह भेजना बंद किया गया।
qs-done-empty-title = कुछ भी नहीं भेजा गया।
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
qs-receiving-title = फ़ाइलें आ रही हैं।
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
qs-pair-old-label-title = अभी आपके लैपटॉप का नाम
qs-pair-old-rename-label = आपके लैपटॉप का नाम
qs-pair-old-step-2 = अगर कोई सूची दिखे, तो अपने लैपटॉप का नाम चुनें।
qs-pair-old-locked-body = पाँच गलत कोड के बाद, आपके लैपटॉप को सुरक्षित रखने के लिए जोड़ना बंद हो गया। कुछ भी शेयर नहीं हुआ। अगर यह आप थे, तो नए कोड के लिए “फिर से शुरू करें” चुनें।
qs-pair-new-needs-update = उस पर PCTwin का पुराना संस्करण है। उस पर PCTwin अपडेट करें, फिर से खोजें।
qs-pairing-no-network = आपका लैपटॉप किसी Wi-Fi या तार वाले नेटवर्क पर नहीं है। उसे जोड़ें, फिर से कोशिश करें।
qs-problem-room-title = { $folder } में पर्याप्त जगह नहीं है।
qs-problem-room-body = इन फ़ाइलों को { $needed } चाहिए, और वहाँ { $room } इस्तेमाल हो सकता है। कुछ भी सहेजा नहीं गया। जगह खाली करें या कोई दूसरा फ़ोल्डर चुनें, फिर से कोशिश करें।
qs-done-refused-title = पाने वाले लैपटॉप ने ये फ़ाइलें नहीं लीं।
qs-done-refused-body = हो सकता है उस पर पर्याप्त जगह न हो। कुछ भी नहीं भेजा गया।
