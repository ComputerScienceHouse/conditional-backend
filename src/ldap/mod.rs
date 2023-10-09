use self::user::LdapUser;
use crate::ldap::client::LdapClient;
use crate::ldap::search::SearchAttrs;
use ldap3::{ResultEntry, SearchEntry};
use log::{log, Level};

pub mod client;
pub mod search;
pub mod user;

pub async fn get_intro_members(client: &LdapClient) -> Vec<LdapUser> {
    get_group_members(client, "intromembers").await
}

pub async fn get_active_upperclassmen(client: &LdapClient) -> Vec<LdapUser> {
    let res = ldap_search(
        client,
        "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
        format!("(&(memberOf=*active*)(!(memberOf=*intromember*)))").as_str(),
        None,
    )
    .await;

    res.iter()
        .map(|r| {
            let user = SearchEntry::construct(r.to_owned());
            LdapUser::from_entry(&user)
        })
        .collect()
}

pub async fn get_group_members(client: &LdapClient, group: &str) -> Vec<LdapUser> {
    let res = ldap_search(
        client,
        "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
        format!("memberOf=*{}*", group).as_str(),
        None,
    )
    .await;

    res.iter()
        .map(|r| {
            let user = SearchEntry::construct(r.to_owned());
            LdapUser::from_entry(&user)
        })
        .collect()
}

pub async fn get_user(client: &LdapClient, user: &str) -> Vec<LdapUser> {
    let res = ldap_search(
        client,
        "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
        format!("(uid={})", user).as_str(),
        None,
    )
    .await;
    res.iter()
        .map(|r| {
            let user = SearchEntry::construct(r.to_owned());
            LdapUser::from_entry(&user)
        })
        .collect()
}

async fn ldap_search(
    client: &LdapClient,
    ou: &str,
    query: &str,
    attrs: Option<SearchAttrs>,
) -> Vec<ResultEntry> {
    log!(Level::Debug, "LDAP Search with query {query} from {ou}");
    let attrs = attrs.unwrap_or_default().finalize();
    let mut ldap = client.ldap.get().await.unwrap();
    ldap.with_timeout(std::time::Duration::from_secs(5));
    let (results, _result) = ldap
        .search(ou, ldap3::Scope::Subtree, query, attrs)
        .await
        .unwrap()
        .success()
        .unwrap();

    results
}
