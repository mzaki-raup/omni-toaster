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
# $path ialah folder laporan ranap.
error-crash-loop = OmniToaster berhenti dimulakan semula selepas ranap berulang kali. Laporan ranap ada dalam { $path }.

## Nama bahasa, ditunjukkan dalam Tetapan ▸ Umum ▸ Bahasa. Setiap nama
## ditulis dalam bahasanya sendiri.

language-system = Sama seperti Windows
language-en-us = English
language-ms-my = Bahasa Melayu

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
