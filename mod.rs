mod login;
mod sidebar;
mod topbar;
mod pg_genel;
mod pg_farming;
mod pg_yetenek;
mod pg_esya;
mod pg_captcha;
mod pg_ocr;
mod pg_client;
mod pg_ayarlar;
mod pg_kayitlar;
mod pg_pm;

pub use login::render_login;
pub use sidebar::render_sidebar;
pub use topbar::render_topbar;
pub use pg_genel::page_genel_bakis;
pub use pg_farming::page_farming;
pub use pg_yetenek::page_yetenekler;
pub use pg_esya::page_esyalar;
pub use pg_captcha::page_captcha;
pub use pg_ocr::page_ocr;
pub use pg_client::page_coklu_client;
pub use pg_ayarlar::page_ayarlar;
pub use pg_kayitlar::page_kayitlar;
pub use pg_pm::page_auto_pm;

