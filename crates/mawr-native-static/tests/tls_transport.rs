use std::sync::Arc;

use mawr_core::{AbsoluteUrl, NavigationFailureKind, OperationFailure, SessionId};
use mawr_native_static::{
    CancellationToken, DerCertificate, DestinationPolicy, NativeStaticConfig, NativeStaticEngine,
    NavigationRequest, TlsTrust,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

const CA_PEM: &[u8] = include_bytes!("fixtures/tls-ca.pem");
const SERVER_CERT_PEM: &[u8] = include_bytes!("fixtures/tls-server.pem");
const SERVER_KEY_PEM: &[u8] = include_bytes!("fixtures/tls-server-key.pem");

struct TlsFixture {
    port: u16,
    task: JoinHandle<()>,
}

impl TlsFixture {
    async fn spawn() -> Self {
        let certificates = CertificateDer::pem_slice_iter(SERVER_CERT_PEM)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let key = PrivateKeyDer::from_pem_slice(SERVER_KEY_PEM).unwrap();
        let server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certificates, key)
            .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        return;
                    };
                    let mut request = vec![0_u8; 4096];
                    let Ok(count) = stream.read(&mut request).await else {
                        return;
                    };
                    if count == 0 {
                        return;
                    }
                    let response = b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 9\r\n\r\ntls-works";
                    let _ = stream.write_all(response).await;
                    let _ = stream.shutdown().await;
                });
            }
        });
        Self { port, task }
    }

    fn url(&self) -> AbsoluteUrl {
        AbsoluteUrl::new(format!("https://localhost:{}/", self.port)).unwrap()
    }

    fn trusted_engine(&self) -> NativeStaticEngine {
        let ca = CertificateDer::from_pem_slice(CA_PEM).unwrap();
        let trust =
            TlsTrust::only(vec![DerCertificate::new(ca.as_ref().to_vec()).unwrap()]).unwrap();
        NativeStaticEngine::new(
            NativeStaticConfig::default()
                .with_destination_policy(DestinationPolicy::loopback(self.port).unwrap())
                .with_tls_trust(trust),
        )
    }
}

impl Drop for TlsFixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[tokio::test]
async fn https_validates_hostname_and_explicit_test_root() {
    let fixture = TlsFixture::spawn().await;
    let session = fixture
        .trusted_engine()
        .start_session(SessionId::new(1).unwrap());
    let result = session
        .navigate(
            NavigationRequest::get(fixture.url()),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(result.body(), b"tls-works");
}

#[tokio::test]
async fn untrusted_tls_chain_fails_closed() {
    let fixture = TlsFixture::spawn().await;
    let engine = NativeStaticEngine::new(
        NativeStaticConfig::default()
            .with_destination_policy(DestinationPolicy::loopback(fixture.port).unwrap()),
    );
    let error = engine
        .start_session(SessionId::new(2).unwrap())
        .navigate(
            NavigationRequest::get(fixture.url()),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        error,
        OperationFailure::NavigationFailure(NavigationFailureKind::SecureConnection)
    );
}
