import * as path from 'node:path';
import * as fs from 'node:fs';

/**
 * Environment configuration interface
 */
export interface EnvironmentConfig {
  description: string;
  awsAccount: string;
  awsRegion: string;
  frontendVersion: string;
  twitchClientId: string;
  githubOwner: string;
  tags: Record<string, string>;
}

/**
 * Environments configuration file structure
 */
interface EnvironmentsConfig {
  environments: Record<string, EnvironmentConfig>;
  default: string;
}

/**
 * Load environment configuration from config file
 */
export function loadEnvironmentConfig(environmentName?: string): {
  name: string;
  config: EnvironmentConfig;
} {
  // Allow config path to be overridden via environment variable for testing
  const configPath = process.env.CDK_ENVIRONMENTS_CONFIG 
    ? path.resolve(process.env.CDK_ENVIRONMENTS_CONFIG)
    : path.join(__dirname, '../../config/environments.json');
  
  let environmentsConfig: EnvironmentsConfig;
  
  try {
    const configContent = fs.readFileSync(configPath, 'utf-8');
    environmentsConfig = JSON.parse(configContent);
  } catch (error) {
    throw new Error(`Failed to load environment configuration from ${configPath}: ${error}`);
  }

  // Determine which environment to use
  const envName = environmentName || 
                  process.env.ENVIRONMENT || 
                  environmentsConfig.default;

  if (!environmentsConfig.environments[envName]) {
    throw new Error(
      `Environment "${envName}" not found in configuration. Available environments: ${Object.keys(environmentsConfig.environments).join(', ')}`
    );
  }

  return {
    name: envName,
    config: environmentsConfig.environments[envName],
  };
}

/**
 * Get stack name with environment suffix
 */
export function getStackName(baseName: string, environmentName: string): string {
  // Production environment uses base name without suffix for backward compatibility
  if (environmentName === 'production') {
    return baseName;
  }
  return `${baseName}-${environmentName}`;
}

/**
 * Get resource name with environment prefix
 */
export function getResourceName(baseName: string, environmentName: string): string {
  // Production environment uses base name without prefix for backward compatibility
  if (environmentName === 'production') {
    return baseName;
  }
  return `${environmentName}-${baseName}`;
}

/**
 * Get IAM role name with environment suffix
 * Returns undefined for production (let CDK generate name for backward compatibility)
 */
export function getRoleName(baseName: string, environmentName: string): string | undefined {
  if (environmentName === 'production') {
    return undefined; // Use CDK-generated name for production
  }
  return `${baseName}-${environmentName}`;
}

/**
 * Get all available environment names
 */
export function getAvailableEnvironments(): string[] {
  const configPath = path.join(__dirname, '../../config/environments.json');
  const configContent = fs.readFileSync(configPath, 'utf-8');
  const environmentsConfig: EnvironmentsConfig = JSON.parse(configContent);
  return Object.keys(environmentsConfig.environments);
}
