use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use derivative::Derivative;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct StyleView {
    pub icon: Vec<u8>,
    pub name: String,
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct CharacterView {
    pub name: String,
    pub policy: String,
    pub styles: Vec<StyleView>,
}

#[async_trait]
pub trait TtsService: std::fmt::Debug + Send + Sync {
    async fn tts(&self, style_id: &str, text: &str) -> Result<Vec<u8>>;
    async fn styles(&self) -> Result<Vec<CharacterView>>;
}

#[derive(Derivative)]
#[derivative(Debug)]
struct TtsServicesInner {
    #[allow(clippy::type_complexity)]
    services: RwLock<HashMap<String, (Box<dyn TtsService>, Vec<CharacterView>)>>,
}

#[derive(Clone, Debug)]
pub struct TtsServices {
    inner: Arc<TtsServicesInner>,
}

impl TtsServices {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(TtsServicesInner {
                services: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub async fn styles(&self) -> HashMap<String, Vec<CharacterView>> {
        let services = self.inner.services.read().await;

        let mut styles = HashMap::new();

        for (id, (_service, service_styles)) in services.iter() {
            styles.insert(id.clone(), service_styles.clone());
        }

        styles
    }

    pub async fn register(&self, service_id: &str, service: Box<dyn TtsService>) -> Result<()> {
        let mut services = self.inner.services.write().await;

        if services.get(service_id).is_some() {
            anyhow::bail!("'{service_id}' is already taken");
        }

        let styles = service.styles().await?;

        services.insert(service_id.to_owned(), (service, styles));

        Ok(())
    }

    pub async fn is_available(&self, service_id: &str, style_id: &str) -> bool {
        let services = self.inner.services.read().await;

        let Some((_service, styles)) = services.get(service_id) else {
            return false;
        };

        styles
            .iter()
            .flat_map(|s| s.styles.iter())
            .any(|style| style.id == style_id)
    }

    pub async fn tts(&self, service_id: &str, style_id: &str, text: &str) -> Result<Vec<u8>> {
        let services = self.inner.services.read().await;

        let Some((service, _styles)) = services.get(service_id) else {
            anyhow::bail!("'{service_id}' is not registered");
        };

        service.tts(style_id, text).await
    }
}
