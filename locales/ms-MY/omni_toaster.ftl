# OmniToaster — Bahasa Melayu (ms-MY)
#
# Ejaan dan istilah mengikut Dewan Bahasa dan Pustaka (DBP).
#
# Setiap kunci di sini mesti ada dalam en-US juga, dalam commit yang sama;
# ujian parity kunci akan gagal jika tidak. Teks Melayu boleh sampai 30%
# lebih panjang daripada bahasa Inggeris, jadi susun atur diuji dengan
# rentetan Melayu yang paling panjang (docs/10-i18n.md).
#
# Sunting fail ini dengan editor, jangan sekali-kali dengan heredoc shell:
# heredoc tanpa petikan akan mengembangkan { $pemboleh ubah } dan
# backtick, lalu membuang mesej itu tanpa amaran (docs/GOTCHAS.md).

## Permulaan dan ralat maut. Ini satu-satunya teks yang pengguna boleh
## lihat sebelum tetingkap tetapan ada, jadi ia mesti lengkap dalam kedua
## bahasa.

# $path ialah folder yang cuba digunakan oleh aplikasi.
error-data-dir-not-writable = OmniToaster tidak dapat dimulakan kerana ia tidak boleh menulis ke { $path }.
error-data-dir-advice = Pindahkan aplikasi ke folder yang anda boleh tulis, atau mulakannya dengan --data-dir yang menunjuk ke folder sedemikian. Tiada apa-apa yang diubah.
# $path ialah folder yang gagal dibuat.
error-data-dir-not-creatable = OmniToaster tidak dapat membuat folder datanya { $path }.
error-exe-location-unknown = OmniToaster tidak dapat mengenal pasti lokasi pemasangannya.
# $path ialah folder tempat aplikasi menyimpan lognya.
notice-log-location = Log disimpan dalam { $path }.
# Ditunjukkan apabila salinan kedua dimulakan. Bukan ralat: aplikasi yang
# diminta pengguna sudah berjalan, jadi salinan kedua berundur dengan senyap.
notice-already-running = OmniToaster sudah berjalan. Cari ikonnya dalam kawasan pemberitahuan.
# $path ialah folder laporan ranap.
error-crash-loop = OmniToaster berhenti dimulakan semula selepas ranap berulang kali. Laporan ranap ada dalam { $path }.

## --help. Dicetak dalam bahasa pengguna, kerana orang yang menjalankan
## OmniToaster --help pada mesin berbahasa Melayu ialah orang yang paling
## memerlukannya dalam bahasa Melayu.

help-usage = Penggunaan: OmniToaster [pilihan]
help-intro = Pemberitahuan toast mudah alih untuk peringatan, waktu solat, cuaca dan masa.
help-option-help = tunjukkan bantuan ini dan keluar
help-option-version = tunjukkan versi dan keluar
help-option-self-test = jalankan pemeriksaan tanpa antara muka dan keluar, satu baris bagi setiap pemeriksaan
help-option-worker = jalankan sebagai proses pekerja (belum dilaksanakan; Fasa 8)
help-option-data-dir = gunakan folder ini dan bukan OmniToasterData di sebelah fail boleh laksana
help-option-log-level = log pada tahap ini bagi larian ini: error, warn, info, debug atau trace
help-option-lang = gunakan bahasa ini bagi larian ini, mengatasi tetapan Windows
help-footer = Semua yang ditulis oleh OmniToaster kekal dalam folder datanya. Tiada apa-apa dihantar ke mana-mana melainkan anda menghidupkannya.

## Nama bahasa, ditunjukkan dalam Tetapan ▸ Umum ▸ Bahasa. Setiap nama
## ditulis dalam bahasanya sendiri.

language-system = Sama seperti Windows
language-en-us = English
language-ms-my = Bahasa Melayu

## Toast. Huruf biasa untuk teks, Huruf Besar Setiap Perkataan untuk tindakan.
## Hanya label terbina dalam ada di sini: teks peringatan pengguna bukan kunci.

toast-app-name = OmniToaster
toast-test-title = Toast Ujian
toast-test-body = Beginilah rupa toast dengan tetapan anda sekarang.
# Butang dalam baris tindakan toast: menangguhkan peringatan selama tempoh yang ditetapkan dalam Tetapan ▸ Peringatan. Dikelabukan pada toast yang tiada apa-apa untuk ditunda.
toast-action-snooze = Tunda
# Butang dalam baris tindakan toast yang menutup toast itu. "Tutup", bukan "Tolak": pemberitahuan ditutup, bukan ditolak.
toast-action-dismiss = Tutup
# Butang dalam baris tindakan toast yang membawa Tetapan ke hadapan.
toast-action-open = Buka
# Ditunjukkan dalam baris tindakan di sebelah Tutup, apabila peringatan telah ditunda sebanyak yang dibenarkan oleh Tetapan ▸ Peringatan.
toast-snooze-limit = Tiada lagi tundaan untuk peringatan ini.
# Tajuk satu-satunya toast yang memberitahu peringatan terlepas semasa PC dimatikan atau tidur.
toast-missed-title = Peringatan Terlepas
# Isi toast itu. "Buka" ialah butang Buka pada toast, yang menunjukkan senarainya.
toast-missed-body = Beberapa peringatan tiba masanya semasa PC dimatikan atau tidur. Buka untuk melihatnya.

## Tempat toast muncul: sembilan kedudukan pratetap — empat bucu, empat tepi
## dan tengah — serta kedudukan tersuai. Ditunjukkan dalam
## Tetapan ▸ Penampilan.

toast-position-top-left = Kiri Atas
toast-position-top = Atas
toast-position-top-right = Kanan Atas
toast-position-left = Kiri
toast-position-centre = Tengah
toast-position-right = Kanan
toast-position-bottom-left = Kiri Bawah
toast-position-bottom = Bawah
toast-position-bottom-right = Kanan Bawah
toast-position-custom = Kedudukan Tersuai
# Di bawah pemilih kedudukan, apabila Kedudukan Tersuai dipilih.
toast-position-custom-hint = Seret pratonton untuk meletakkannya, atau taipkan jarak dari tepi kiri dan atas dalam peratus.

## Sebab imej yang diimport ditolak. Huruf biasa, dan setiap satu menyatakan
## peraturan yang dilanggar oleh fail itu. $name ialah nama fail itu sendiri.

error-import-too-large = { $name } lebih besar daripada 5 MB, jadi ia tidak diimport.
error-import-not-an-image = { $name } bukan gambar PNG, JPEG atau WebP, walau apa pun namanya.
error-import-too-many-pixels = { $name } lebih besar daripada 4096 kali 4096 piksel, jadi ia tidak diimport.
error-import-undecodable = { $name } tidak dapat dibaca sebagai gambar. Ia mungkin rosak.
error-import-escapes = { $name } tidak dapat disalin ke dalam folder OmniToaster.
error-import-unreadable = { $name } tidak dapat dibaca.

## Bentuk toast, ditunjukkan dalam Tetapan ▸ Penampilan.

toast-shape-rectangle = Segi Empat Tepat
toast-shape-rounded = Segi Empat Bucu Bulat
toast-shape-pill = Pil

## Jenis kaca, menu lungsur Kaca dalam Tetapan ▸ Penampilan.

# Tiada kaca: latar belakang toast padu.
toast-glass-none = Tiada
# Desktop di belakang toast, dikaburkan, dengan hampir tiada warna toast.
toast-glass-blur = Kabur
# Desktop yang dikaburkan di bawah warna toast, separa lut sinar.
toast-glass-acrylic = Akrilik
# Warna toast, kebanyakannya legap, dengan sedikit desktop kelihatan.
toast-glass-mica-like = Seperti Mika

## Animasi toast, menu lungsur Animasi Masuk dan Animasi Keluar dalam
## Tetapan ▸ Animasi. Huruf Besar Setiap Perkataan: ia item senarai.

# Toast menggelongsor masuk dari luar tepi kanan skrin.
toast-effect-slide-from-right = Gelongsor dari Kanan
# Toast menggelongsor naik dari bawah kawasan kerja skrin.
toast-effect-slide-from-bottom = Gelongsor dari Bawah
# Toast mengalir keluar dari satu titik di bawah skrin, sempit dahulu, seperti jin keluar dari pelita.
toast-effect-genie-from-bottom = Jin dari Bawah
# Toast membesar dari tengahnya, terlebih sedikit dengan pelonggaran pegas.
toast-effect-pop-out = Timbul
# Toast menggelongsor keluar dari tepi skrin tempat ia berada, seperti laci.
toast-effect-drawer = Laci
# Toast muncul atau hilang secara pudar. Inilah juga yang dimainkan apabila gerakan dikurangkan.
toast-effect-appear = Muncul
# Toast terbentuk, atau hancur, butir demi butir.
toast-effect-dissolve = Larut
# Lingkaran cahaya dalam warna aksen toast mengembang di sekelilingnya lalu reda.
toast-effect-glow = Bercahaya
# Teks muncul dahulu; latar belakang menjadi padu di belakangnya.
toast-effect-alpha-to-solid = Lut Sinar ke Padu
# Keluar sahaja: toast mengalir masuk ke ikon dulang, seperti Jin secara terbalik.
toast-effect-shrink-to-tray = Kecut ke Dulang

## Cara kemajuan animasi dibentuk sepanjang tempohnya, menu lungsur Pelonggaran.

# Kelajuan yang sama sepanjang masa.
toast-easing-linear = Linear
# Laju pada mulanya, perlahan dengan lembut pada akhirnya.
toast-easing-ease-out = Perlahan di Hujung
# Terlebih sedikit lalu reda, seperti pegas.
toast-easing-spring = Pegas

## Ikon dulang dan menunya. Huruf Besar Setiap Perkataan untuk setiap item,
## kerana ia item menu. Glif di hadapan setiap satu dipilih dalam kod.

tray-open-settings = Buka Tetapan
tray-pause = Jeda Semua Pemberitahuan
tray-pause-30 = Selama 30 Minit
tray-pause-60 = Selama 1 Jam
tray-pause-until-resumed = Sehingga Saya Sambung Semula
tray-resume = Sambung Semula Pemberitahuan
tray-test-toast = Uji Toast
tray-snooze-last = Tunda Yang Terakhir
tray-quit = Keluar daripada OmniToaster

## Teks apung dulang. Huruf biasa: ia petunjuk, bukan label. Ia membawa
## keadaan jeda, kerana aplikasi yang pengguna lupa telah disenyapkan ialah
## akibat paling buruk sesuatu jeda.

tray-tooltip-idle = OmniToaster sedang berjalan.
# $time ialah masa jeda itu tamat, diformat oleh ot_i18n.
tray-tooltip-paused-until = Pemberitahuan dijeda sehingga { $time }.
tray-tooltip-paused-until-resumed = Pemberitahuan dijeda sehingga anda menyambungnya semula.

## Keluar semasa jeda sedang berjalan. Huruf biasa untuk teks; butang
## menggunakan Huruf Besar Setiap Perkataan dan menyatakan tindakannya.

tray-quit-confirm-title = Keluar daripada OmniToaster?
tray-quit-confirm-body = Pemberitahuan sedang dijeda. Keluar sekarang akan menutup OmniToaster sepenuhnya, dan tiada apa-apa akan dihantar sehingga anda memulakannya semula.
tray-quit-confirm-quit = Tetap Keluar
tray-quit-confirm-stay = Teruskan Berjalan

## Tetingkap tetapan: tajuknya, bahagian dalam rel kiri, dan panel Umum.
## Huruf Besar Setiap Perkataan untuk label, huruf biasa untuk petunjuk.

settings-window-title = Tetapan
settings-section-general = Umum
settings-section-appearance = Penampilan
# Bahagian pustaka gambar: gambar dimuat naik sekali dan digunakan oleh mana-mana pemberitahuan.
settings-section-pictures = Pustaka Gambar
# Bahagian yang menetapkan cara toast muncul dan hilang.
settings-section-animation = Animasi
# Bahagian yang memegang peringatan dan cara Tunda berfungsi.
settings-section-reminders = Peringatan

## Tetapan ▸ Peringatan: bahagian Tunda. Huruf Besar Setiap Perkataan untuk
## label dan item senarai, huruf biasa untuk petunjuk.

# Senarai juntai: berapa lama satu tekanan Tunda menangguhkan peringatan.
settings-snooze-length = Tempoh Tunda
# Petunjuk di bawah Tempoh Tunda.
settings-snooze-length-hint = Apa yang dilakukan oleh satu tekanan Tunda pada toast peringatan.
# Pilihan Tempoh Tunda.
settings-snooze-5-minutes = 5 Minit
# Pilihan Tempoh Tunda.
settings-snooze-15-minutes = 15 Minit
# Pilihan Tempoh Tunda.
settings-snooze-1-hour = 1 Jam
# Pilihan Tempoh Tunda.
settings-snooze-1-day = 1 Hari
# Pilihan Tempoh Tunda: tempoh yang ditaip oleh pengguna.
settings-snooze-custom = Tersuai
# Medan untuk tempoh tunda tersuai, ditunjukkan apabila Tersuai dipilih. Unitnya dalam label, bukan selepas nombor.
settings-snooze-custom-length = Tempoh Tersuai (Minit)
# Gelangsar: berapa kali toast satu peringatan boleh ditunda.
settings-snooze-limit = Had Tundaan
# Petunjuk di bawah Had Tundaan.
settings-snooze-limit-hint = Selepas sebanyak ini tundaan, toast hanya menawarkan Tutup.
# Gelangsar: berapa jam ke belakang peringatan yang terlepas semasa PC dimatikan atau tidur masih disenaraikan. Unitnya dalam label.
settings-missed-window = Tempoh Susulan (Jam)
# Petunjuk di bawah Tempoh Susulan.
settings-missed-window-hint = Peringatan yang terlepas semasa PC dimatikan atau tidur disenaraikan jika ia sebaru ini; yang lebih lama digugurkan.
# Tajuk senarai peringatan yang terlepas, di bahagian atas Tetapan ▸ Peringatan.
settings-missed-heading = Peringatan Terlepas
# Keadaan kosong senarai peringatan terlepas.
settings-missed-empty = Tiada peringatan yang terlepas.
# Di bawah tajuk peringatan yang terlepas. $time ialah tarikh dan masanya, sudah diformat.
settings-missed-due = Sepatutnya { $time }

## Tetapan ▸ Animasi. Huruf Besar Setiap Perkataan untuk label, huruf biasa
## untuk petunjuk.

# Cara toast muncul.
settings-animation-enter = Animasi Masuk
# Cara toast hilang.
settings-animation-leave = Animasi Keluar
# Berapa lama setiap animasi, dalam milisaat.
settings-animation-duration = Tempoh
# Cara kemajuan animasi dibentuk sepanjang tempohnya.
settings-animation-easing = Pelonggaran
# Kotak semak: mainkan setiap animasi sebagai pudar.
settings-animation-reduce = Kurangkan Gerakan
# Di bawah Kurangkan Gerakan, apabila tetapan animasi Windows dihidupkan.
settings-animation-windows-on = Windows menunjukkan animasi, jadi animasi yang dipilih di sini dimainkan kecuali Kurangkan Gerakan ditanda.
# Di bawah Kurangkan Gerakan, apabila tetapan animasi Windows dimatikan.
settings-animation-windows-off = Animasi Windows dimatikan, jadi setiap animasi dimainkan sebagai pudar.
# Di bawah Kurangkan Gerakan, apabila ia ditanda.
settings-animation-reduced = Setiap animasi dimainkan sebagai pudar.
# Butang: menunjukkan toast ujian dengan animasi yang dipilih di atas.
settings-animation-play = Mainkan

## Tetapan ▸ Penampilan. Huruf Besar Setiap Perkataan untuk label, huruf biasa
## untuk petunjuk.

settings-toast-preview = Pratonton
settings-toast-test = Tunjukkan Toast Ujian
settings-toast-shape = Bentuk
settings-toast-radius = Jejari Bucu
settings-toast-width = Lebar
settings-toast-opacity = Kelegapan
settings-toast-border = Sempadan
settings-toast-shadow = Bayang
# Di bawah kotak semak Bayang apabila ia dikelabukan kerana bentuknya bukan Segi Empat Tepat.
settings-toast-shadow-hint = Windows melukis bayang hanya untuk bentuk Segi Empat Tepat.
# Di bawah kotak semak Bayang apabila ia dikelabukan kerana kaca dihidupkan.
settings-toast-shadow-glass-hint = Windows tidak melukis bayang di belakang kaca.
# Label menu lungsur yang memilih jenis kaca.
settings-toast-glass = Kaca
# Di bawah menu lungsur Kaca apabila ia dikelabukan kerana bentuknya bukan Segi Empat Tepat.
settings-toast-glass-shape-hint = Kaca hanya tersedia untuk bentuk Segi Empat Tepat: Windows mengaburkan seluruh petak di belakang toast, melepasi bucu bulat.
# Di bawah menu lungsur Kaca apabila satu jenis kaca dipilih: pratonton tidak dapat mengaburkan apa yang di belakang tetingkap tetapan.
settings-toast-glass-hint = Pratonton menunjukkan warna, bukan kesan kabur. Tunjukkan toast ujian untuk melihat kesan kabur.
settings-toast-background = Latar Belakang
settings-toast-accent = Warna Aksen
settings-toast-header = Pengepala
settings-toast-header-icon = Ikon
settings-toast-header-app-name = Nama Aplikasi
settings-toast-header-time = Masa
settings-toast-position = Kedudukan
settings-toast-position-custom = Titik Tersuai
# Berapa lama toast kekal pada skrin, dalam saat (3 hingga 60).
settings-toast-duration = Tempoh Paparan
# Petunjuk di bawah peluncur itu.
settings-toast-duration-hint = Menuding pada toast menjeda kiraan masanya sehingga penuding beralih.
# Berapa banyak toast boleh dipaparkan serentak (1 hingga 5); yang lain menunggu giliran.
settings-toast-max-visible = Dipaparkan Serentak
# Ruang antara satu toast dengan toast seterusnya dalam timbunan, dalam piksel.
settings-toast-stack-gap = Jarak Antara Toast
# Label pemilih yang menentukan monitor tempat toast dipaparkan.
settings-toast-monitor = Monitor
# Petunjuk di bawah pemilih itu.
settings-toast-monitor-hint = Toast dipaparkan pada monitor ini. Apabila ia tidak disambungkan, toast dipaparkan pada monitor utama.
# Pilihan pertama dalam pemilih monitor, dan lalai: monitor yang ditetapkan sebagai utama oleh Windows.
settings-toast-monitor-primary = Monitor Utama
# Satu monitor dalam pemilih. $number ialah nombor paparan Windows, $name namanya sendiri, $width dan $height saiznya dalam piksel.
settings-toast-monitor-option = { $number }: { $name } ({ $width } × { $height })
# Sama, untuk monitor yang kini menjadi monitor utama.
settings-toast-monitor-option-primary = { $number }: { $name } ({ $width } × { $height }), Utama
# Nama monitor yang tidak melaporkan nama, seperti kebanyakan skrin komputer riba. $number ialah nombor paparannya.
settings-toast-monitor-unnamed = Paparan { $number }
# Monitor yang dipilih apabila ia tidak disambungkan. $name ialah namanya.
settings-toast-monitor-missing = { $name } (Tidak Disambungkan)
settings-toast-position-x = Dari Kiri
settings-toast-position-y = Dari Atas
settings-toast-map-hint = Klik titik untuk memilih kedudukan, atau seret untuk meletakkan toast di mana-mana pada skrin.
settings-toast-test-content = Kandungan Toast Ujian
settings-toast-test-content-hint = Ini hanya mengubah pratonton dan toast ujian. Setiap pemberitahuan membawa ikon, imej dan warna aksennya sendiri.
settings-toast-icon = Ikon
settings-toast-image = Imej
settings-toast-image-border = Sempadan Sekeliling Imej
settings-toast-own-accent = Warna Aksen Sendiri
# Tajuk dialog Buka Windows.
settings-toast-icon-dialog-title = Pilih ikon untuk pengepala toast
settings-toast-image-dialog-title = Pilih imej untuk toast
# Penapis jenis fail dalam dialog Buka.
settings-toast-image-filter = Gambar (PNG, JPEG, WebP)
# Tetapan > Pustaka Gambar. Baris di bawah tajuk bahagian.
settings-library-hint = Muat naik gambar sekali dan gunakannya dalam mana-mana pemberitahuan, sebagai ikon atau imejnya.
# Butang yang membuka dialog fail untuk menambah gambar ke pustaka.
settings-library-upload = Muat Naik Gambar
# Item terakhir setiap pemilih ikon dan imej, yang memuat naik gambar dan memilihnya.
settings-library-upload-new = Muat Naik Gambar Baharu
# Tajuk dialog Buka apabila memuat naik dari bahagian Pustaka Gambar.
settings-library-dialog-title = Pilih gambar untuk pustaka
# Dipaparkan semasa pustaka sedang dibaca.
settings-library-loading = Memuatkan gambar…
# Dipaparkan apabila pustaka belum ada gambar.
settings-library-empty = Belum ada gambar. Muat naik satu untuk digunakan dalam pemberitahuan.
# Petua pada medan nama di sebelah setiap gambar.
settings-library-label-hint = Nama gambar ini dalam setiap pemilih
# Butang yang membuang gambar daripada pustaka.
settings-library-remove = Buang
# Ditanya sebelum membuang gambar yang masih digunakan. $label ialah nama gambar.
settings-library-in-use = “{ $label }” masih digunakan. Buang juga? Apa-apa yang menggunakannya akan kembali kepada ikon aplikasi atau tanpa imej.
# Baris di bawah soalan itu, apabila toast ujian di bawah Penampilan memaparkan gambar tersebut.
settings-library-used-by-test-toast = Toast ujian menggunakannya.
# Baris di bawah soalan itu, apabila templat pemberitahuan menggunakan gambar tersebut.
settings-library-used-by-templates = Templat pemberitahuan yang menggunakannya akan kehilangannya.
# Mengesahkan pembuangan itu.
settings-library-remove-anyway = Buang Juga
# Membatalkan pembuangan itu.
settings-library-keep = Simpan
# Item pertama pemilih ikon: tiada gambar sendiri, jadi ikon aplikasi.
settings-library-none-icon = Ikon Aplikasi
# Item pertama pemilih imej: tiada imej.
settings-library-none-image = Tiada Imej
# Tajuk senarai gambar yang disertakan bersama aplikasi, dalam Tetapan > Pustaka Gambar.
settings-library-builtin = Gambar Terbina Dalam
# Baris di bawah tajuk itu.
settings-library-builtin-hint = Gambar ini disertakan bersama OmniToaster. Gunakannya untuk mana-mana pemberitahuan; ia tidak boleh dinamakan semula atau dibuang.
# Tajuk kumpulan dalam pemilih gambar dan Pustaka Gambar: gambar terbina dalam untuk waktu solat, cuaca dan jenis peringatan.
settings-builtin-group-prayer = Waktu Solat
settings-builtin-group-weather = Cuaca
settings-builtin-group-reminder = Peringatan
# Nama gambar terbina dalam, dipaparkan dalam pemilih dan Pustaka Gambar. Waktu solat membawa kedua-dua nama, seperti di seluruh Tetapan.
settings-builtin-masjid-imsak = Masjid · Imsak
settings-builtin-masjid-subuh = Masjid · Subuh (Fajr)
settings-builtin-masjid-dhuha = Masjid · Dhuha
settings-builtin-masjid-zohor = Masjid · Zohor (Dhuhr)
settings-builtin-masjid-asar = Masjid · Asar (Asr)
settings-builtin-masjid-maghrib = Masjid · Maghrib
settings-builtin-masjid-isyak = Masjid · Isyak (Isha)
# Gambar cuaca.
settings-builtin-sunny = Cerah
settings-builtin-partly-cloudy = Separa Berawan
settings-builtin-cloudy = Berawan
settings-builtin-hazy = Berjerebu
settings-builtin-fog = Berkabus
settings-builtin-drizzle = Hujan Renyai
settings-builtin-downpour = Hujan Lebat
settings-builtin-thunderstorm = Ribut Petir
settings-builtin-windy = Berangin
# Gambar jenis peringatan.
settings-builtin-alarm-clock = Jam Loceng
settings-builtin-meeting = Mesyuarat
settings-builtin-training = Latihan
settings-language = Bahasa
# Di bawah pemilih bahasa. Penukaran berlaku serta-merta, jadi petunjuk ini
# menerangkan satu-satunya perkara yang pengguna tidak dapat teka: apa yang
# diikuti oleh "Sama seperti Windows".
settings-language-hint = Sama seperti Windows mengikut bahasa paparan Windows anda. Pilihan di sini digunakan serta-merta dan diingati.
settings-theme = Tema
settings-startup = Mula dengan Windows
settings-startup-hint = OmniToaster dimulakan apabila anda menandatangani masuk. Folder ini boleh dipindahkan — tetapan akan mengikutinya.
# Ditunjukkan apabila nilai pendaftaran menunjuk ke salinan OmniToaster yang lain.
settings-startup-elsewhere = Salinan OmniToaster yang lain ditetapkan untuk mula dengan Windows, jadi salinan ini tidak.

## Baris status tetapan. Huruf biasa; di sinilah kebenaran bukan modal
## tinggal, jadi setiap mesej menyatakan apa yang kini benar.

settings-status-ready = Sedia.
# $time ialah masa jeda itu tamat, diformat oleh ot_i18n.
settings-status-paused-until = Pemberitahuan dijeda sehingga { $time }.
settings-status-paused-until-resumed = Pemberitahuan dijeda sehingga anda menyambungnya semula.
settings-status-resumed = Pemberitahuan dihidupkan semula.
settings-status-tray-failed = Ikon dulang tidak dapat dibuat, jadi menutup tetingkap ini ialah satu-satunya jalan keluar: { $reason }
settings-status-language-failed = Bahasa tidak dapat ditukar: { $reason }
# Baris status selepas muat naik. $label ialah nama gambar dalam pustaka.
settings-status-library-added = “{ $label }” telah ditambah ke pustaka gambar.
# Baris status apabila gambar yang dimuat naik sudah ada dalam pustaka. $label ialah namanya yang sedia ada.
settings-status-library-already-there = Gambar itu sudah ada dalam pustaka, sebagai “{ $label }”.
# Baris status selepas gambar dibuang daripada pustaka.
settings-status-library-removed = Gambar telah dibuang daripada pustaka.
# Baris status apabila pustaka tidak dapat dibaca atau diubah. $reason ialah punca teknikalnya.
settings-status-library-failed = Pustaka gambar tidak dapat dikemas kini: { $reason }
settings-status-dialog-failed = Dialog fail tidak dapat dibuka: { $reason }
settings-status-saved = Tetapan anda telah disimpan.
settings-status-autostart-on = OmniToaster akan mula dengan Windows.
settings-status-autostart-off = OmniToaster tidak akan mula dengan Windows.
# $reason ialah ralat asas daripada sistem pengendalian, dalam bahasa Inggeris.
settings-status-save-failed = Tetapan anda tidak dapat disimpan: { $reason }
settings-status-autostart-failed = Tetapan permulaan tidak dapat diubah: { $reason }

## Nama tema, ditunjukkan dalam Tetapan ▸ Umum ▸ Tema, mengikut susunan
## pemilihnya. Gelap ialah tetapan lalai; Sistem mengikut Windows.

settings-theme-system = Sistem
settings-theme-dark = Gelap
settings-theme-light = Cerah

## Tetapan ▸ Umum ▸ Jam: cara masa ditulis di mana-mana.

# Label senarai juntai.
settings-clock = Jam
# Petunjuk senarai juntai.
settings-clock-hint = Automatik ialah 24 jam dalam Bahasa Melayu dan mengikut Windows dalam bahasa Inggeris.
# Pilihan Jam.
settings-clock-automatic = Automatik
# Pilihan Jam.
settings-clock-12 = 12 Jam
# Pilihan Jam.
settings-clock-24 = 24 Jam

## Pola tarikh, masa dan nombor. Hanya ot_i18n::format_datetime membaca
## pola ini; tiada tempat lain memformat tarikh atau nombor
## (docs/10-i18n.md).

format-date-short = { $day }/{ $month }/{ $year }
format-time-24h = { $hour }:{ $minute }
format-time-12h = { $hour }:{ $minute } { $period }
format-period-am = pagi
format-period-pm = petang
# $date dan $time sudah diformat oleh pola di atas.
format-date-time = { $date } pada { $time }
# Tarikh pendek apabila tetapan serantau Windows meletakkan bulan dahulu. Tidak digunakan dalam bahasa Melayu, yang sentiasa d/m/yyyy; ada di sini supaya kedua-dua fail mempunyai kunci yang sama.
format-date-short-month-first = { $day }/{ $month }/{ $year }
# Tempoh sebelum masa peringatan. $count ialah nombor. Bahasa Melayu tiada bentuk jamak.
format-before-minutes = { $count } minit sebelum
# Tempoh sebelum masa peringatan.
format-before-hours = { $count } jam sebelum
# Tempoh sebelum masa peringatan.
format-before-days = { $count } hari sebelum
# Tempoh sebelum masa peringatan.
format-before-weeks = { $count } minggu sebelum
# Cara peringatan berulang. $count ialah nombor. Huruf kecil: ia berada di dalam ayat.
format-every-days = { $count ->
    [1] setiap hari
   *[other] setiap { $count } hari
}
# Cara peringatan berulang.
format-every-weeks = { $count ->
    [1] setiap minggu
   *[other] setiap { $count } minggu
}
# Cara peringatan mingguan berulang. $days ialah senarai nama hari yang sudah digabungkan.
format-every-weeks-on = { $count ->
    [1] setiap minggu pada { $days }
   *[other] setiap { $count } minggu pada { $days }
}
# Cara peringatan berulang.
format-every-months = { $count ->
    [1] setiap bulan
   *[other] setiap { $count } bulan
}
# Cara peringatan berulang.
format-every-years = { $count ->
    [1] setiap tahun
   *[other] setiap { $count } tahun
}
# Dua item senarai: "Isnin dan Khamis".
format-list-pair = { $first } dan { $second }
# Tiga atau lebih: item pertama, kemudian selebihnya yang sudah digabungkan — "Isnin, Selasa dan Khamis".
format-list-more = { $first }, { $rest }

## Nama hari, digunakan oleh format_datetime dan bukan oleh locale OS,
## supaya nama hari Melayu betul walaupun Windows ditetapkan ke bahasa
## Inggeris.

day-monday = Isnin
day-tuesday = Selasa
day-wednesday = Rabu
day-thursday = Khamis
day-friday = Jumaat
day-saturday = Sabtu
day-sunday = Ahad

## Nama bulan.

month-january = Januari
month-february = Februari
month-march = Mac
month-april = April
month-may = Mei
month-june = Jun
month-july = Julai
month-august = Ogos
month-september = September
month-october = Oktober
month-november = November
month-december = Disember

## Reminder types. Each key is also the stable identifier stored in the
## database (category.type_key), so a key is never renamed. The name is a list
## item, so Title Case. The -body line is the toast's text when neither the
## reminder nor its template has a body of its own: sentence case. A user's
## own title and body are never keys and are never translated.

# The type of a reminder for a dated event: a talk, a course, a gathering.
reminder-event = Acara
# Toast body for an Event reminder that has no text of its own.
reminder-event-body = Satu acara akan bermula.
# The type of a reminder for something the user has to do.
reminder-task = Tugasan
# Toast body for a Task reminder that has no text of its own.
reminder-task-body = Satu tugasan perlu disiapkan.
# The type of a reminder for a yearly date: a birthday, a wedding anniversary.
reminder-anniversary = Ulang Tahun
# Toast body for an Anniversary reminder that has no text of its own.
reminder-anniversary-body = Ingat ulang tahun ini.
# Fatigue reminder: rest the eyes from the screen.
reminder-fatigue-eye = Rehat Mata
# Toast body for an Eye Break reminder that has no text of its own.
reminder-fatigue-eye-body = Rehatkan mata: pandang sesuatu yang jauh.
# Fatigue reminder: check how you are sitting.
reminder-fatigue-posture = Postur
# Toast body for a Posture reminder that has no text of its own.
reminder-fatigue-posture-body = Duduk tegak dan longgarkan bahu anda.
# Fatigue reminder: drink water.
reminder-fatigue-hydration = Minum Air
# Toast body for a Drink Water reminder that has no text of its own.
reminder-fatigue-hydration-body = Masa untuk segelas air.
# Fatigue reminder: the working day has gone on long enough.
reminder-fatigue-work-hours = Had Waktu Bekerja
# Toast body for a Work-Hour Limit reminder that has no text of its own.
reminder-fatigue-work-hours-body = Anda telah mencapai had waktu bekerja.
# The type of a reminder for a countdown.
reminder-timer = Pemasa
# Toast body for a Timer reminder that has no text of its own.
reminder-timer-body = Pemasa anda telah tamat.
# The type of a reminder that wakes or alerts: the most urgent kind.
reminder-alarm = Penggera
# Toast body for an Alarm reminder that has no text of its own.
reminder-alarm-body = Penggera anda sedang berbunyi.
# The type of a reminder for a bill or other payment.
reminder-payment = Pembayaran
# Toast body for a Payment reminder that has no text of its own.
reminder-payment-body = Satu pembayaran perlu dibuat.
# The type of a reminder for a meeting with someone: a doctor, a client.
reminder-appointment = Temu Janji
# Toast body for an Appointment reminder that has no text of its own.
reminder-appointment-body = Anda ada temu janji.
# The type of a reminder that fits none of the others.
reminder-custom = Tersuai
# Toast body for a Custom reminder that has no text of its own.
reminder-custom-body = Ini peringatan anda.

## Tetapan ▸ Peringatan: senarai dan penyunting. Huruf Besar Setiap Perkataan
## untuk label, butang dan item senarai; huruf biasa untuk petunjuk, keadaan
## kosong, soalan dan mesej pengesahan.

# Tajuk di atas senarai peringatan.
settings-reminders-heading = Peringatan Anda
# Butang yang memulakan peringatan baharu dalam penyunting.
settings-reminders-new = Peringatan Baharu
# Ditunjukkan semasa senarai sedang dibaca.
settings-reminders-loading = Memuatkan peringatan anda…
# Keadaan kosong senarai.
settings-reminders-empty = Anda belum ada peringatan. Peringatan Baharu akan membuatnya.
# Baris kedua dalam senarai. $type ialah jenis peringatan; $time bila ia seterusnya muncul, sudah diformat.
settings-reminder-row-next = { $type } · seterusnya { $time }
# Baris kedua bagi peringatan yang ditunda. $time ialah bila tundaan tamat, sudah diformat.
settings-reminder-row-snoozed = { $type } · ditunda hingga { $time }
# Baris kedua bagi peringatan yang tidak akan muncul lagi.
settings-reminder-row-done = { $type } · selesai
# Baris kedua semasa masa seterusnya sedang dikira.
settings-reminder-row-plain = { $type }
# Baris kedua bagi peringatan berulang. $repeat ialah cara ia berulang ("setiap 2 minggu pada Isnin"); $time bila ia seterusnya muncul.
settings-reminder-row-next-repeat = { $type } · { $repeat } · seterusnya { $time }
# Ayat di bawah Ulang dalam penyunting. $repeat ialah cara ia berulang.
settings-reminder-repeat-summary = Berulang { $repeat }.
# Tajuk bahagian keletihan dalam Tetapan ▸ Peringatan: rehat mata, postur, minum air, had waktu bekerja.
settings-fatigue-heading = Peringatan Keletihan
# Selepas nombor rehat mata, postur atau minum. $count ialah nombor itu.
settings-fatigue-minutes-apart = minit sekali
# Selepas nombor had waktu bekerja. $count ialah nombor itu.
settings-fatigue-hours-of-work = jam bekerja
# Petunjuk di bawah bahagian keletihan.
settings-fatigue-hint = Setiap satu dikira dari saat anda mula menggunakan PC. Sejam atau lebih jauh daripadanya memulakan hari semula.
# Medan penyunting: tajuk peringatan itu sendiri.
settings-reminder-title = Tajuk
# Medan penyunting: teks peringatan itu sendiri, ditunjukkan dalam toastnya.
settings-reminder-body = Butiran
# Medan penyunting: jenis peringatan.
settings-reminder-type = Jenis
# Medan penyunting: hari yang dimaksudkan.
settings-reminder-date = Tarikh
# Medan penyunting: masa dalam hari.
settings-reminder-time = Masa
# Medan penyunting: toast sebelum masa itu sendiri.
settings-reminder-offsets = Ingatkan Saya Sebelum
# Menambah baris pada Ingatkan Saya Sebelum.
settings-reminder-offset-add = Tambah Masa Sebelum
# Membuang satu baris Ingatkan Saya Sebelum.
settings-reminder-offset-remove = Buang
# Petunjuk di bawah Ingatkan Saya Sebelum.
settings-reminder-offsets-hint = Setiap satu ialah toast tersendiri, selain toast pada masa itu sendiri.
# Unit, dalam senarai juntai Ingatkan Saya Sebelum dan Ulang.
settings-unit-minutes = Minit
# Unit.
settings-unit-hours = Jam
# Unit.
settings-unit-days = Hari
# Unit.
settings-unit-weeks = Minggu
# Unit.
settings-unit-months = Bulan
# Unit.
settings-unit-years = Tahun
# Medan penyunting: cara peringatan berulang.
settings-reminder-repeat = Ulang
# Pilihan Ulang: ia muncul sekali sahaja.
settings-repeat-never = Tidak Berulang
# Medan penyunting: setiap berapa unit Ulang — "Ulang Setiap 2" dengan Minggu bermaksud setiap dua minggu.
settings-reminder-every = Ulang Setiap
# Medan penyunting: hari dalam minggu bagi ulangan mingguan.
settings-reminder-weekdays = Pada
# Petunjuk di bawah hari dalam minggu.
settings-reminder-weekdays-hint = Jika tiada yang ditanda, ia berulang pada hari dalam minggu tarikh itu sendiri.
# Butang penyunting.
settings-reminder-save = Simpan
# Butang penyunting: keluar tanpa menyimpan.
settings-reminder-cancel = Batal
# Butang penyunting, dan butang yang mengesahkan soalan Padam.
settings-reminder-delete = Padam
# Butang yang membatalkan soalan Padam atau Kosongkan Sejarah.
settings-reminder-keep = Kekalkan
# Soalan yang ditanya oleh Padam dahulu.
settings-reminder-delete-confirm = Padam peringatan ini? Sejarahnya kekal sehingga luput.
# Tajuk sejarah peringatan yang sedang disunting.
settings-reminder-history = Sejarah
# Keadaan kosong sejarah.
settings-reminder-history-empty = Belum ada apa-apa berlaku pada peringatan ini.
# Butang yang mengosongkan sejarah peringatan yang disunting, dan butang yang mengesahkan soalannya.
settings-reminder-history-clear = Kosongkan Sejarah
# Soalan yang ditanya oleh Kosongkan Sejarah dahulu.
settings-reminder-history-clear-confirm = Kosongkan sejarah peringatan ini? Ini tidak boleh dibuat asal.
# Satu baris sejarah. $time ialah bila, sudah diformat; $action apa yang berlaku.
settings-reminder-history-row = { $time } — { $action }
# Apa yang berlaku pada satu paparan, dalam sejarah.
settings-history-shown = Dipaparkan
# Apa yang berlaku pada satu paparan.
settings-history-snoozed = Ditunda
# Apa yang berlaku pada satu paparan.
settings-history-dismissed = Ditutup
# Apa yang berlaku pada satu paparan.
settings-history-opened = Dibuka
# Apa yang berlaku pada satu paparan.
settings-history-missed = Terlepas
# Dikatakan di bawah Tajuk apabila ia kosong.
error-reminder-title-empty = Beri peringatan ini tajuk.
# Dikatakan di bawah Masa apabila setiap paparan sudah berlalu.
error-reminder-time-passed = Masa ini telah berlalu dan peringatan ini tidak berulang, jadi ia tidak akan muncul.
# Baris status apabila peringatan tidak dapat dibaca atau disimpan. $reason ialah puncanya.
settings-status-reminders-failed = Peringatan anda tidak dapat dibaca atau disimpan: { $reason }
# Label sumber waktu solat, sentiasa dipaparkan di sebelah waktu hari ini (Tetapan > Solat, tip alat dulang). Waktu diambil daripada JAKIM e-Solat. $time ialah masa ia diambil, diformat oleh pemformat kongsi.
prayer-source-jakim = Sumber: JAKIM e-Solat, diambil { $time }
# Label yang sama apabila waktu datang daripada salinan tersimpan kerana pengambilan tidak dapat atau tidak perlu dibuat. $time ialah masa salinan itu diambil.
prayer-source-jakim-cached = Sumber: JAKIM e-Solat (salinan tersimpan), diambil { $time }
# Label sumber waktu solat apabila waktu datang daripada set data yang disertakan dalam aplikasi.
prayer-source-bundled = Data luar talian
# Label sumber waktu solat apabila waktu dikira kerana pengguna memilihnya, atau lokasi berada di luar Malaysia.
prayer-source-calculated = Dikira
# Label yang sama apabila lokasi berada di dalam Malaysia, tempat waktu JAKIM sendiri boleh berbeza beberapa minit.
prayer-source-calculated-approximate = Dikira (anggaran)
# Label sumber waktu solat apabila waktu JAKIM dikehendaki tetapi tidak dapat diperoleh, maka aplikasi mengiranya.
prayer-source-calculated-fallback = Dikira (sandaran)
# Label sandaran yang sama apabila lokasi berada di dalam Malaysia, tempat pengiraan boleh berbeza daripada JAKIM beberapa minit.
prayer-source-calculated-fallback-approximate = Dikira (sandaran, anggaran)
# Nama bahagian Solat dalam senarai di sebelah kiri tetingkap tetapan.
settings-section-prayer = Solat
# Nama bahagian Cuaca dalam senarai di sebelah kiri tetingkap tetapan.
settings-section-weather = Cuaca
# Tajuk kumpulan lokasi dalam Tetapan > Solat.
prayer-location-heading = Lokasi
# Soalan dalam kad larian pertama yang bertanya sebelum aplikasi mencari tempat pengguna daripada alamat internetnya.
prayer-consent-title = Kesan lokasi anda daripada alamat internet anda?
# Penerangan di bawah soalan itu. ipapi.co ialah nama perkhidmatan carian dan kekal seperti ditulis.
prayer-consent-body = OmniToaster boleh mencari di mana anda berada sekali sehari, untuk memilih zon solat anda. Alamat internet anda dihantar kepada ipapi.co dan tidak disimpan. Anda boleh menaip tempat sebagai ganti.
# Butang yang bersetuju dengan carian itu.
prayer-consent-allow = Benarkan
# Butang yang menolak carian itu buat masa ini; pengguna masih boleh mengubah fikiran kemudian.
prayer-consent-decline = Bukan Sekarang
# Kotak semak yang menghidupkan atau mematikan carian harian selepas jawapan pertama.
prayer-detect-checkbox = Kesan lokasi saya daripada alamat internet saya
# Petunjuk di bawah kotak semak itu.
prayer-detect-hint = Sekali sehari, dengan kebenaran anda sahaja. Tempat yang anda tetapkan sendiri tidak akan diganti.
# Dipaparkan semasa carian sedang berjalan.
prayer-location-looking = Sedang mencari lokasi anda…
# Dipaparkan apabila perkhidmatan carian lokasi tidak dapat dihubungi.
prayer-error-network = Perkhidmatan lokasi tidak dapat dihubungi. Semak sambungan anda, atau taip tempat sebagai ganti.
# Dipaparkan apabila perkhidmatan lokasi menolak permintaan, contohnya kerana ditanya terlalu kerap.
prayer-error-service = Perkhidmatan lokasi tidak memberi jawapan. Cuba lagi kemudian, atau taip tempat sebagai ganti.
# Dipaparkan apabila perkhidmatan lokasi menjawab sesuatu yang tidak dapat digunakan.
prayer-error-answer = Perkhidmatan lokasi memberi jawapan yang tidak dapat digunakan. Taip tempat sebagai ganti.
# Tempat tersimpan apabila ia dijumpai daripada alamat internet. $place ialah bandar dan wilayah.
prayer-location-detected = Lokasi: { $place } (dikesan daripada alamat internet anda)
# Tempat tersimpan apabila pengguna memilihnya. $place ialah bandar dan wilayah, atau koordinat.
prayer-location-manual = Lokasi: { $place } (ditetapkan oleh anda)
# Dipaparkan apabila belum ada tempat tersimpan.
prayer-location-none = Belum ada lokasi ditetapkan. Cari bandar atau taip koordinat.
# Butang yang membuka kawalan memasukkan tempat.
prayer-location-change = Tukar Lokasi
# Butang yang menutup kawalan memasukkan tempat.
prayer-close = Tutup
# Label medan carian bandar.
prayer-search-label = Bandar
# Petunjuk kelabu dalam medan carian bandar yang kosong.
prayer-search-placeholder = Cari bandar
# Butang yang menjalankan carian bandar.
prayer-search-button = Cari
# Dipaparkan semasa carian bandar sedang berjalan.
prayer-search-loading = Sedang mencari…
# Dipaparkan apabila carian bandar tidak menemui apa-apa.
prayer-search-empty = Tiada bandar sepadan dengan carian itu.
# Satu hasil carian bandar. $place ialah bandar dan wilayah; $zone ialah nama zon waktunya.
prayer-search-result = { $place } ({ $zone })
# Label medan koordinat.
prayer-coords-label = Koordinat
# Petunjuk kelabu dalam medan koordinat yang kosong.
prayer-coords-placeholder = Latitud, longitud, contohnya 3.14, 101.69
# Butang yang menggunakan koordinat yang ditaip.
prayer-coords-use = Guna Koordinat
# Dipaparkan apabila koordinat yang ditaip bukan dua nombor.
prayer-coords-error-format = Taip latitud dan longitud, seperti 3.14, 101.69.
# Dipaparkan apabila koordinat yang ditaip berada di luar julat Bumi.
prayer-coords-error-range = Latitud ialah dari -90 hingga 90 dan longitud dari -180 hingga 180.
# Dipaparkan apabila zon waktu PC ini tidak dapat dibaca untuk koordinat di luar Malaysia.
prayer-coords-error-zone = Zon waktu PC ini tidak dapat dibaca. Cari bandar sebagai ganti.
# Dipaparkan selepas koordinat di luar Malaysia digunakan. $zone ialah zon waktu PC ini, yang diandaikan.
prayer-coords-zone-note = Zon waktu: { $zone }, diambil daripada PC ini. Semak sama ada ia betul untuk tempat ini, atau cari bandar sebagai ganti.
# Label senarai juntai sumber.
prayer-source-label = Sumber
# Pilihan pertama dalam senarai juntai sumber: JAKIM di dalam Malaysia dan dikira di tempat lain.
prayer-choice-auto = Automatik
# Pilihan kedua dalam senarai juntai sumber.
prayer-choice-jakim = JAKIM e-Solat
# Pilihan ketiga dalam senarai juntai sumber.
prayer-choice-calculated = Dikira
# Petunjuk di bawah senarai juntai sumber.
prayer-source-hint = Automatik menggunakan JAKIM e-Solat di dalam Malaysia dan mengira di tempat lain.
# Amaran yang dipaparkan apabila JAKIM dipilih tetapi tiada zon untuk ditanya.
prayer-warning-needs-zone = JAKIM memerlukan zon. Pilih satu di bawah, atau gunakan Automatik.
# Amaran yang dipaparkan apabila JAKIM dipilih untuk tempat di luar Malaysia.
prayer-warning-abroad = Tempat ini di luar Malaysia. Waktu JAKIM mengikut zon yang anda pilih dan mungkin berbeza daripada amalan tempatan.
# Label baris zon.
prayer-zone-label = Zon
# Zon yang dijumpai daripada tempat tersimpan. $code ialah kod zon JAKIM seperti SGR01; $areas ialah senarai kawasan; $state ialah negeri.
prayer-zone-detected = Zon dikesan: { $code } · { $areas } · { $state }
# Zon yang dipilih pengguna. Pemegang tempat yang sama seperti zon yang dikesan.
prayer-zone-chosen = Zon: { $code } · { $areas } · { $state }
# Dipaparkan apabila belum ada zon.
prayer-zone-none = Belum ada zon. Tetapkan lokasi di Malaysia, atau pilih zon.
# Butang yang membuka senarai zon.
prayer-zone-change = Tukar Zon
# Butang yang kembali kepada zon yang dijumpai daripada tempat.
prayer-zone-use-detected = Guna Zon Dikesan
# Petunjuk kelabu dalam medan carian zon yang kosong.
prayer-zone-search-placeholder = Cari mengikut kod, negeri atau kawasan
# Dipaparkan apabila carian zon tidak menemui apa-apa.
prayer-zone-no-match = Tiada zon sepadan dengan carian itu.
# Satu baris senarai zon. $code ialah kod zon, $state ialah negeri, $areas ialah senarai kawasan.
prayer-zone-row = { $code } · { $state } · { $areas }
# Label senarai juntai kaedah pengiraan.
prayer-method-label = Kaedah
# Satu kaedah pengiraan.
prayer-preset-mwl = Liga Dunia Islam (Muslim World League)
# Satu kaedah pengiraan.
prayer-preset-isna = ISNA
# Satu kaedah pengiraan.
prayer-preset-umm-al-qura = Umm al-Qura
# Satu kaedah pengiraan yang mengikut sudut JAKIM tetapi bukan waktu JAKIM sendiri.
prayer-preset-malaysia = Malaysia (Sudut JAKIM, Anggaran)
# Label senarai juntai mazhab Asar. Asar ialah solat waktu petang.
prayer-asr-label = Mazhab Asar
# Satu mazhab Asar; yang diikuti JAKIM.
prayer-asr-shafii = Syafie
# Satu mazhab Asar.
prayer-asr-hanafi = Hanafi
# Petunjuk di bawah senarai juntai mazhab Asar.
prayer-asr-hint = Syafie ialah ikutan JAKIM. Hanafi meletakkan Asar kira-kira sejam lebih lewat.
# Tajuk senarai waktu solat hari ini.
prayer-today-heading = Waktu Solat Hari Ini
# Dipaparkan semasa waktu hari ini sedang dikira.
prayer-today-loading = Sedang mengira waktu hari ini…
# Dipaparkan apabila tiada sumber dapat memberi waktu hari ini.
prayer-today-nothing = Tiada waktu solat tersedia. Tetapkan lokasi, atau semak sambungan anda dan cuba lagi.
# Dipaparkan sebelum ada lokasi untuk dijadikan asas.
prayer-today-none-yet = Pilih lokasi untuk melihat waktu hari ini.
# Satu solat dalam senarai hari itu: kedua-dua nama, supaya pembaca mana-mana nama dapat mencarinya. Subuh ialah solat waktu fajar.
prayer-name-fajr = Subuh (Fajr)
# Syuruk, yang menamatkan waktu solat Subuh.
prayer-name-syuruk = Syuruk (Sunrise)
# Solat waktu tengah hari.
prayer-name-dhuhr = Zohor (Dhuhr)
# Solat waktu petang.
prayer-name-asr = Asar (Asr)
# Solat waktu matahari terbenam.
prayer-name-maghrib = Maghrib
# Nama solat Jumaat dalam bahasa Melayu, dengan nama Inggeris di sebelahnya, dalam senarai solat dan pemberitahuannya.
prayer-name-jumuah = Jumaat (Jumu'ah)
# Solat waktu malam.
prayer-name-isha = Isyak (Isha)
# Tajuk kumpulan tetapan pemberitahuan dalam Tetapan > Solat.
prayer-notify-heading = Pemberitahuan
# Kotak semak yang memberi Syuruk (matahari terbit) pemberitahuan tersendiri. Lima waktu solat sehari sentiasa ada satu.
prayer-notify-syuruk = Beritahu pada Syuruk (Sunrise)
# Kotak semak yang memberi Jumaat (solat Jumaat) pemberitahuan. Pada hari Jumaat ia menggantikan pemberitahuan Zohor, pada masa yang sama.
prayer-notify-jumuah = Beritahu untuk Jumaat (Gantikan Zohor pada Hari Jumaat)
# Petunjuk di bawah kotak semak Syuruk.
prayer-notify-hint = Lima waktu solat sehari sentiasa mempunyai pemberitahuan.
# Label medan nombor bagi berapa lama solat yang terlepas boleh disenaraikan.
prayer-azan-window-label = Tempoh Solat Terlepas (Minit)
# Petunjuk di bawah medan itu.
prayer-azan-window-hint = Solat yang terlepas semasa PC dimatikan disenaraikan jika tidak lebih lama daripada ini. Yang lebih lama digugurkan.
# Label medan bilangan minit sebelum solat peringatannya datang.
prayer-before-label = Ingatkan Sebelum (Minit)
# Petunjuk di bawah kotak semak "ingatkan sebelum" bagi setiap solat.
prayer-before-hint = Tandakan sesuatu solat untuk mendapat peringatan ringkas sekian minit sebelumnya. Peringatan yang akan datang selepas solat bermula dilangkau.
# Kandungan pemberitahuan solat. $prayer ialah nama solat dengan kedua-dua nama, seperti Subuh (Fajr).
prayer-toast-body = Masuk waktu { $prayer }.
# Kandungan peringatan sebelum solat. $prayer ialah nama solat dengan kedua-dua nama, seperti Subuh (Fajr). $count ialah bilangan minit. Bahasa Melayu tiada bentuk jamak.
prayer-before-body = { $prayer } dalam { $count } minit.
# Tajuk pemberitahuan yang dipaparkan, paling banyak sekali sehari, apabila waktu solat tidak dapat dikira.
prayer-toast-unavailable-title = Tiada waktu solat tersedia
# Kandungan pemberitahuan itu.
prayer-toast-unavailable-body = OmniToaster tidak dapat mengira waktu solat hari ini. Buka Tetapan dan pilih lokasi.
# Tajuk kumpulan tempat pemberitahuan setiap solat boleh diubah.
prayer-template-heading = Templat Solat
# Label senarai juntai yang memilih templat solat mana yang disunting.
prayer-template-prayer = Solat
# Label medan tajuk pemberitahuan solat.
prayer-template-title = Tajuk
# Label medan mesej pemberitahuan solat.
prayer-template-message = Mesej
# Kotak semak yang memberi pemberitahuan solat warna aksen tersendiri.
prayer-template-accent-use = Guna warna aksen tersuai
# Label senarai juntai gambar pemberitahuan solat.
prayer-template-picture = Gambar
# Pilihan pertama dalam senarai itu: gambar yang disertakan dengan aplikasi untuk solat ini.
prayer-template-picture-builtin = Gambar terbina dalam
# Petunjuk di bawah medan templat.
prayer-template-hint = Biarkan medan kosong untuk menggunakan teks terbina dalam dalam bahasa semasa. Teks yang anda tulis disimpan seperti yang ditaip dan tidak pernah diterjemah.
# Butang yang menyimpan templat.
prayer-template-save = Simpan Templat
# Butang yang mengosongkan tajuk dan mesej supaya teks terbina dalam digunakan semula.
prayer-template-default = Guna Teks Lalai
# Butang yang menunjukkan pemberitahuan seperti rupanya nanti.
prayer-template-preview = Pratonton Pemberitahuan
# Dipaparkan selepas templat disimpan.
prayer-template-saved = Templat disimpan.
# Label senarai juntai yang memilih apa yang dimainkan oleh sesuatu solat apabila tiba waktunya.
prayer-sound-label = Bunyi
# Pilihan bunyi: loceng pendek yang disertakan dengan aplikasi. Yang lalai.
prayer-sound-chime = Loceng Pendek
# Pilihan bunyi: azan penuh yang disertakan dengan aplikasi.
prayer-sound-azan = Azan Penuh
# Pilihan bunyi: klip audio yang dipilih pengguna daripada failnya sendiri.
prayer-sound-file = Fail Audio Saya
# Pilihan bunyi: tiada bunyi langsung.
prayer-sound-none = Senyap
# Butang yang membuka dialog untuk memilih klip audio.
prayer-sound-choose = Pilih Fail Audio
# Dipaparkan selagi belum ada klip audio dipilih.
prayer-sound-none-chosen = Belum ada fail dipilih.
# Butang yang memainkan bunyi solat itu sekarang, untuk didengar.
prayer-sound-play = Main Bunyi
# Butang yang menghentikan bunyi.
prayer-sound-stop = Henti
# Tajuk dialog untuk memilih klip audio.
prayer-sound-dialog-title = Pilih klip audio
# Nama penapis jenis fail dalam dialog itu.
prayer-sound-filter = Audio (WAV, MP3, Ogg)
# Dinyatakan apabila klip audio melebihi had saiz.
error-import-audio-too-large = Klip itu melebihi 20 MB.
# Dinyatakan apabila fail yang dipilih bukan fail audio jenis yang dimainkan aplikasi, apa jua namanya.
error-import-not-audio = Fail itu bukan klip WAV, MP3 atau Ogg Vorbis, apa jua namanya.
# Dinyatakan apabila klip audio berjalan lebih daripada sepuluh minit.
error-import-audio-too-long = Klip itu berjalan lebih daripada sepuluh minit.
# Nama keadaan cuaca: langit cerah atau kebanyakannya cerah. Dipaparkan dalam Tetapan dan dalam pemberitahuan cuaca.
weather-clear = Cerah
# Nama keadaan cuaca: separa berawan atau mendung.
weather-cloudy = Berawan
# Nama keadaan cuaca: renyai, hujan atau hujan lebat sekejap-sekejap.
weather-rain = Hujan
# Nama keadaan cuaca: ribut petir.
weather-storm = Ribut petir
# Nama keadaan cuaca: salji, butir salji atau hujan salji sekejap-sekejap.
weather-snow = Salji
# Nama keadaan cuaca: kabus.
weather-fog = Kabus
# Nama yang dipaparkan apabila perkhidmatan cuaca menghantar keadaan yang tidak dikenali aplikasi ini. Tidak pernah perkataan Inggeris daripada perkhidmatan itu.
weather-unknown = Keadaan tidak jelas
# Kandungan pemberitahuan apabila cuaca berubah. $from dan $to ialah nama keadaan (weather-rain dan seterusnya).
weather-toast-condition-body = Cuaca berubah daripada { $from } kepada { $to }.
# Tajuk pemberitahuan apabila suhu jatuh dengan cepat.
weather-toast-fall-title = Suhu sedang menurun
# Kandungan pemberitahuan itu. $degrees ialah bilangan darjah Celsius penuh ia jatuh, $now suhu sekarang, dan $count bilangan jam ia jatuh. Bahasa Melayu tiada bentuk jamak.
weather-toast-fall-body = Suhu { $degrees } °C lebih sejuk berbanding { $count } jam lalu, kini { $now } °C.
# Tajuk pemberitahuan apabila suhu naik dengan cepat.
weather-toast-rise-title = Suhu sedang meningkat
# Kandungan pemberitahuan itu. $degrees ialah bilangan darjah Celsius penuh ia naik, $now suhu sekarang, dan $count bilangan jam ia naik.
weather-toast-rise-body = Suhu { $degrees } °C lebih panas berbanding { $count } jam lalu, kini { $now } °C.
# Tajuk Tetapan > Cuaca.
weather-heading = Pemberitahuan Cuaca
# Kotak semak yang menghidupkan pemberitahuan cuaca. Dimatikan secara lalai.
weather-enable = Beritahu saya apabila cuaca berubah
# Petunjuk di bawah kotak semak itu: apa yang dihantar, dan bahawa apabila dimatikan tiada apa-apa dihantar.
weather-enable-hint = Menghantar lokasi yang disimpan di bawah Solat, dibundarkan kepada kira-kira satu kilometer, kepada Open-Meteo. Apabila dimatikan, tiada apa-apa dihantar.
# Label baris yang menamakan lokasi untuk cuaca.
weather-place-label = Lokasi
# Dipaparkan apabila belum ada lokasi disimpan.
weather-place-none = Tiada lokasi ditetapkan. Tetapkan satu di bawah Solat.
# Petunjuk di bawah lokasi.
weather-place-hint = Cuaca menggunakan lokasi yang sama dengan waktu solat.
# Label medan bilangan minit antara setiap semakan cuaca.
weather-poll-label = Semak Setiap (Minit)
# Petunjuk di bawah medan itu.
weather-poll-hint = Perkhidmatan dikemas kini kira-kira setiap 15 minit, jadi bertanya lebih kerap tidak mengubah apa-apa.
# Tajuk senarai jenis cuaca yang hendak diberitahu.
weather-notify-heading = Beritahu Saya Apabila Menjadi
# Petunjuk di bawah senarai itu.
weather-notify-hint = Perubahan hanya diberitahu setelah cuaca baharu dilihat dua kali berturut-turut, jadi cuaca yang berubah-ubah tidak pernah memberitahu.
# Tajuk tetapan perubahan suhu mendadak.
weather-temperature-heading = Perubahan Suhu Mendadak
# Kotak semak untuk memberitahu suhu jatuh dengan cepat.
weather-fall-enable = Beritahu saya apabila suhu jatuh dengan cepat
# Kotak semak untuk memberitahu suhu naik dengan cepat.
weather-rise-enable = Beritahu saya apabila suhu naik dengan cepat
# Label medan bilangan darjah Celsius yang mesti berubah.
weather-delta-label = Sekurang-kurangnya (°C)
# Label medan bilangan jam perubahan itu mesti berlaku dalamnya.
weather-window-label = Dalam Masa (Jam)
# Petunjuk di bawah tetapan suhu.
weather-temperature-hint = Contohnya 5 °C dalam 3 jam. Ia diberitahu sekali, dan sekali lagi hanya selepas suhu stabil.
# Tajuk templat cuaca.
weather-template-heading = Templat Cuaca
# Label senarai untuk memilih templat keadaan yang hendak disunting.
weather-template-condition = Keadaan
# Dinyatakan apabila tetapan cuaca atau templat tidak dapat dibaca atau disimpan. Puncanya ada dalam log.
weather-error = Tetapan cuaca tidak dapat dibaca atau disimpan.
# Baris kredit di bahagian bawah Tetapan > Cuaca. Data Open-Meteo dilesenkan CC BY 4.0, yang meminta kredit.
weather-credit = Data cuaca oleh Open-Meteo.com, dilesenkan CC BY 4.0.
# Tajuk blok yang menunjukkan cuaca semasa.
weather-now-heading = Cuaca Semasa
# Label baris yang memberi cuaca sekarang.
weather-now-label = Sekarang
# Cuaca semasa. $condition ialah nama keadaan (weather-rain dan seterusnya), $temperature darjah Celsius penuh.
weather-now = { $condition }, { $temperature } °C
# Di bawah cuaca apabila ia terkini. $age ialah berapa lama dahulu, seperti "5 minit lalu" atau "sebentar tadi".
weather-updated = Dikemas kini { $age }.
# Penanda yang dipaparkan apabila cuaca sudah lapuk, contohnya ketika luar talian. $age ialah berapa lama dahulu ia diambil, seperti "3 jam lalu". Tiada perubahan dilaporkan daripada data sedemikian.
weather-stale = Tidak terkini: memaparkan cuaca dari { $age }.
# Dipaparkan apabila cuaca dihidupkan tetapi belum ada yang ditemui.
weather-now-none = Belum ada cuaca ditemui.
# Dipaparkan apabila pemberitahuan cuaca dimatikan.
weather-now-off = Pemberitahuan cuaca dimatikan.
# Berapa lama dahulu, untuk kurang daripada seminit yang lalu.
weather-age-now = sebentar tadi
# Berapa lama dahulu, dalam minit. $count ialah nombor. Bahasa Melayu tiada bentuk jamak.
weather-age-minutes = { $count } minit lalu
# Berapa lama dahulu, dalam jam. $count ialah nombor.
weather-age-hours = { $count } jam lalu
# Berapa lama dahulu, dalam hari. $count ialah nombor.
weather-age-days = { $count } hari lalu
