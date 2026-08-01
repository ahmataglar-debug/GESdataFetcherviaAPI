# Comparison App

iSolarCloud OpenAPI üzerinden GES, inverter ve string ölçümlerini görüntüleyen; string akım/gerilim geçmişini yerelde tutup anomali analizi yapan Tauri 2 + React masaüstü uygulaması.

## Güvenlik modeli

- Uygulama iSolarCloud kullanıcı adı veya parolası istemez.
- Geliştirici `Secret key` ve OAuth tokenları React katmanına geri gönderilmez; işletim sisteminin kimlik bilgisi kasasında saklanır.
- Santral, inverter, şema ve ölçüm verileri kullanıcı profilindeki `~/.comparingapp/comparison.db` SQLite veritabanında tutulur.
- Yalnızca string akım ve gerilim ölçümlerinin geçmişi saklanır. Diğer canlı değerler arayüzde gösterilebilir ama geçmiş analiz veri setine eklenmez.
- Ekran görüntüsünde veya mesajlarda paylaşılmış eski bir Secret kullanılmadan önce geliştirici portalından yenilenmelidir.

## iSolarCloud uygulama ayarı

Developer Portal uygulamasında OAuth yetkilendirmesini açın ve redirect URI olarak tam şu adresi tanımlayın:

```text
http://127.0.0.1:42831/oauth/callback
```

Uygulamada **OpenAPI bağlantısı** ekranına yenilenmiş AppKey ve Secret key değerlerini girin. Portal ayrı bir Application ID göstermediği için OAuth isteğindeki `applicationId` parametresinde AppKey kullanılır. Kaydettikten sonra **Tarayıcıda yetkilendir** düğmesi gerçek iSolarCloud giriş/yetki sayfasını varsayılan tarayıcıda açar. Yerel callback yalnızca kısa süreli yetkilendirme kodunu alır.

Türkiye kurulumları için `Avrupa / Türkiye` bölgesi seçilmelidir. Santral koordinatları Türkiye içinde olduğunda saat dilimi `Europe/Istanbul` olarak belirlenir; koordinat yoksa Türkiye varsayımı kullanılır.

## İlk kullanım

1. OpenAPI ayarlarını kaydedin ve tarayıcı yetkilendirmesini tamamlayın.
2. İlk senkronizasyonla GES ve inverter listesini alın.
3. **GES şemaları** ekranında her inverter stringi için modelin akım ve gerilim point ID değerlerini girin.
4. Fiziksel olarak boş veya ucu takılmamış stringleri `Boş / bağlantısız` işaretleyin.
5. GES’e uygun sıfıra yakın akım/gerilim eşiklerini kaydedin.
6. Yeniden senkronize edin.

iSolarCloud point ID değerleri inverter modeline ve OpenAPI hesabına göre değişebildiği için uygulama bunları tahmin etmez. Hatalı eşleştirme yanlış ölçüm veya eksik veri üretir.

## Analiz ve zamanlama

- Kırmızı alarm yalnızca şemada bağlı bir string, geçerli gündüz/üretim koşulunda hem akım hem gerilim sıfır eşiklerinin altındaysa oluşur.
- Gece ölçümü uyarı üretmez ve 30 günlük öğrenme veri setine katılmaz.
- Sarı şüpheli alarm en az **30 ayrı geçerli gündüz örneği** tamamlandıktan sonra etkinleşir. Değerin hem aynı yerel saat aralığındaki geçmişten hem de çalışan kardeş stringlerden sapması gerekir.
- Normal periyot 24 saattir. Uygulama yeniden açıldığında gecikme 12 saati geçtiyse yakalama senkronizasyonu yapılır. Gece ölçümünden sonra yeniden deneme süresi 8 saate düşer.
- Manuel senkronizasyon canlı görünümü her zaman günceller; henüz periyot dolmadıysa tarihsel analiz örneğini çoğaltmaz.

## Geliştirme

Gereksinimler: güncel Node.js/npm, Rust toolchain ve Tauri 2 için işletim sistemine ait geliştirme paketleri.

```bash
npm install
npm run tauri dev
```

Doğrulama komutları:

```bash
npm run build
cd src-tauri
cargo test
```

Dağıtım paketi:

```bash
npm run tauri build
```

Yalnızca Linux DEB ve RPM çıktıları için:

```bash
npm run tauri build -- --bundles deb,rpm
```

## Modüler yapı

- `src/features`: React ekranları ve kullanıcı akışları
- `src/lib/tauri.ts`: tipli IPC çağrıları; gizli değer döndürmez
- `src-tauri/src/api`: OpenAPI istemcisi ve OAuth callback akışı
- `src-tauri/src/storage`: SQLite şeması ve repository işlemleri
- `src-tauri/src/analysis`: 30 günlük robust sapma modeli
- `src-tauri/src/solar_time.rs`: koordinat ve güneş yüksekliği tabanlı gündüz kontrolü
- `src-tauri/src/scheduler.rs`: 24/12/8 saat zamanlama politikası
- `src-tauri/src/sync_service.rs`: API, kayıt ve analiz orkestrasyonu

Uygulama tarayıcı önizlemesinde örnek veri gösterir. Gerçek OpenAPI ve yerel depolama akışı yalnızca Tauri masaüstü sürecinde çalışır.
