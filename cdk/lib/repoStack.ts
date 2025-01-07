import * as cdk from 'aws-cdk-lib';
import type { Construct } from 'constructs';
import RepoConstruct from './util/repoConstruct';

export default class RepoStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    new RepoConstruct(this, 'RepoConstruct', {
      namespace: 'glowing-telegram',
      names: [
        'crud-lambda',
        'ai-chat-lambda',
        'summarize-transcription-lambda',
        'audio-transcription',
        'video-ingestor',
        'twitch-lambda',
      ],
    });
  }
}
