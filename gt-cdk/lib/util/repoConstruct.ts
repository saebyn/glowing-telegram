import * as cdk from 'aws-cdk-lib';
import { Construct } from 'constructs';
import * as ecr from 'aws-cdk-lib/aws-ecr';

interface RepoConstructProps<T extends string> {
  namespace: string;
  names: T[];
}

type RepoMap<T extends string> = {
  [key in T]: ecr.Repository;
};

export default class RepoConstruct<T extends string> extends Construct {
  public readonly repositoryMap: RepoMap<T> = {} as RepoMap<T>;

  constructor(scope: Construct, id: string, props: RepoConstructProps<T>) {
    super(scope, id);

    for (const name of props.names) {
      this.repositoryMap[name] = new ecr.Repository(this, name, {
        repositoryName: `${props.namespace}/${name}`,
      });

      this.repositoryMap[name].addLifecycleRule({
        maxImageCount: 3,
      });
    }
  }
}
