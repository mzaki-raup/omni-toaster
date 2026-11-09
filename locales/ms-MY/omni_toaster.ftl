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
toast-action-snooze = Tunda
toast-action-dismiss = Tolak
toast-action-open = Buka

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

## Bentuk dan bayang toast, ditunjukkan dalam Tetapan ▸ Penampilan.

toast-shape-rectangle = Segi Empat Tepat
toast-shape-rounded = Segi Empat Bucu Bulat
toast-shape-pill = Pil
toast-shadow-off = Mati
toast-shadow-soft = Lembut
toast-shadow-strong = Kuat

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
