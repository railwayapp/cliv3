use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/schema.graphql",
    query_path = "src/gql/mutations/strings/PluginCreate.graphql",
    response_derives = "Debug"
)]
pub struct PluginCreate;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/schema.graphql",
    query_path = "src/gql/mutations/strings/PluginDelete.graphql",
    response_derives = "Debug"
)]
pub struct PluginDelete;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/schema.graphql",
    query_path = "src/gql/mutations/strings/ValidateTwoFactor.graphql",
    response_derives = "Debug"
)]
pub struct ValidateTwoFactor;
