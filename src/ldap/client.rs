#![allow(unused)]

use async_trait::async_trait;
use deadpool::managed::{self, Metrics};
use ldap3::{drive, Ldap, LdapConnAsync, LdapError, ResultEntry};
use log::{debug, info};
use rand::prelude::SliceRandom;
use rand::SeedableRng;
use std::sync::Arc;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    AsyncResolver,
};

use super::{search::SearchAttrs, user::LdapUser};

type Pool = managed::Pool<LdapManager>;

#[derive(Clone)]
pub struct LdapClient {
    pub(super) ldap: Arc<Pool>,
}

#[derive(Clone)]
pub(super) struct LdapManager {
    ldap_servers: Vec<String>,
    bind_dn: String,
    bind_pw: String,
}

impl LdapManager {
    pub async fn new(bind_dn: &str, bind_pw: &str) -> Self {
        let ldap_servers = get_ldap_servers().await;

        LdapManager {
            ldap_servers,
            bind_dn: bind_dn.to_string(),
            bind_pw: bind_pw.to_string(),
        }
    }
}

#[async_trait]
impl managed::Manager for LdapManager {
    type Type = Ldap;
    type Error = LdapError;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let (conn, mut ldap) = LdapConnAsync::new(
            self.ldap_servers
                .choose(&mut rand::rngs::StdRng::from_entropy())
                .unwrap(),
        )
        .await
        .unwrap();

        drive!(conn);

        ldap.simple_bind(&self.bind_dn, &self.bind_pw)
            .await
            .unwrap();

        Ok(ldap)
    }

    async fn recycle(
        &self,
        ldap: &mut Self::Type,
        _: &Metrics,
    ) -> managed::RecycleResult<Self::Error> {
        ldap.extended(ldap3::exop::WhoAmI).await?;
        Ok(())
    }
}

impl LdapClient {
    pub async fn new(bind_dn: &str, bind_pw: &str) -> Self {
        let ldap_manager = LdapManager::new(bind_dn, bind_pw).await;
        let ldap_pool = Pool::builder(ldap_manager).max_size(5).build().unwrap();

        LdapClient {
            ldap: Arc::new(ldap_pool),
        }
    }
}

async fn get_ldap_servers() -> Vec<String> {
    let resolver = AsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
    let response = resolver.srv_lookup("_ldap._tcp.csh.rit.edu").await.unwrap();

    // TODO: Make sure servers are working
    response
        .iter()
        .map(|record| {
            format!(
                "ldaps://{}",
                record.target().to_string().trim_end_matches('.')
            )
        })
        .collect()
}

impl LdapClient {
    async fn ldap_search(
        &self,
        ou: &str,
        query: &str,
        attrs: Option<SearchAttrs>,
    ) -> anyhow::Result<Vec<ResultEntry>> {
        debug!("LDAP Search with query {query} from {ou}");
        let attrs = attrs.unwrap_or_default().finalize();
        let mut ldap = self.ldap.get().await.unwrap();
        ldap.with_timeout(std::time::Duration::from_secs(5));
        let (results, _result) = ldap
            .search(ou, ldap3::Scope::Subtree, query, attrs)
            .await?
            .success()?;
        Ok(results)
    }
    pub async fn get_group_members(&self, group: &str) -> anyhow::Result<Vec<LdapUser>> {
        let res = self
            .ldap_search(
                "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
                format!("memberOf=*{}*", group).as_str(),
                None,
            )
            .await?;
        Ok(res.iter().map(LdapUser::from).collect())
    }

    pub async fn get_upperclassmen(&self) -> anyhow::Result<Vec<LdapUser>> {
        let res = self
            .ldap_search(
                "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
                format!("(!(memberOf=*10weeks*))").as_str(),
                None,
            )
            .await?;

        Ok(res.iter().map(LdapUser::from).collect())
    }

    pub async fn get_active_upperclassmen(&self) -> anyhow::Result<Vec<LdapUser>> {
        let res = self
            .ldap_search(
                "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
                "(&(memberOf=*active*)(!(memberOf=*intromember*)))"
                    .to_string()
                    .as_str(),
                None,
            )
            .await?;

        Ok(res.iter().map(LdapUser::from).collect())
    }

    pub async fn get_user(&self, user: &str) -> anyhow::Result<Vec<LdapUser>> {
        let res = self
            .ldap_search(
                "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
                format!("(uid={})", user).as_str(),
                None,
            )
            .await?;

        Ok(res.iter().map(LdapUser::from).collect())
    }

    pub async fn get_group_members_exact(&self, group: &str) -> anyhow::Result<Vec<LdapUser>> {
        let res = self
            .ldap_search(
                "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
                format!(
                    "memberOf=cn={},cn=groups,cn=accounts,dc=csh,dc=rit,dc=edu",
                    group
                )
                .as_str(),
                None,
            )
            .await?;

        Ok(res.iter().map(LdapUser::from).collect())
    }

    pub async fn search_users(&self, query: &str) -> anyhow::Result<Vec<LdapUser>> {
        let res = self
            .ldap_search(
                "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
                format!("(|(uid=*{query}*)(cn=*{query}*))").as_str(),
                None,
            )
            .await?;

        Ok(res.iter().map(LdapUser::from).collect())
    }

    pub async fn get_intro_members(&self) -> anyhow::Result<Vec<LdapUser>> {
        self.get_group_members("intromembers").await
    }

    pub async fn get_onfloor_members(&self) -> anyhow::Result<Vec<LdapUser>> {
        Ok(self
            .ldap_search(
                "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
                "(&(roomNumber=*)(memberOf=cn=onfloor,cn=groups,cn=accounts,dc=csh,dc=rit,dc=edu))",
                None,
            )
            .await?
            .iter()
            .map(LdapUser::from)
            .collect())
    }
}
