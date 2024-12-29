import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import * as stepfunctions from 'aws-cdk-lib/aws-stepfunctions';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import APIConstruct from './api';
import UserManagementConstruct from './userManagement';
import DatastoreConstruct from './datastore';

export default class GtCdkStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    const userManagement = new UserManagementConstruct(this, 'UserManagement');

    const dataStore = new DatastoreConstruct(this, 'Datastore');

    const startState = new stepfunctions.Pass(this, 'StartState');
    const simpleStateMachine = new stepfunctions.StateMachine(
      this,
      'SimpleStateMachine',
      {
        definitionBody: stepfunctions.DefinitionBody.fromChainable(startState),
      },
    );

    const openaiSecret = secretsmanager.Secret.fromSecretNameV2(
      this,
      'OpenaiSecret',
      'openai-secret',
    );

    new APIConstruct(this, 'API', {
      streamIngestionFunction: simpleStateMachine,
      userPool: userManagement.userPool,
      userPoolClients: [userManagement.userPoolClient],
      openaiSecret,
      videoMetadataTable: dataStore.videoMetadataTable,
      streamsTable: dataStore.streamsTable,
      streamSeriesTable: dataStore.streamSeriesTable,
      episodesTable: dataStore.episodesTable,
      profilesTable: dataStore.profilesTable,
    });
  }
}
