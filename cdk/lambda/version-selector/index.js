const AWS = require('aws-sdk');

// In-memory cache for version config
let versionCache = {
  version: null,
  timestamp: 0,
  ttl: 60000 // 60 seconds in milliseconds
};

// Environment variables
const BUCKET_NAME = process.env.BUCKET_NAME;
const CONFIG_KEY = 'config/version.json';
const FALLBACK_VERSION = process.env.FALLBACK_VERSION;

/**
 * Lambda@Edge function to dynamically select frontend version
 * This function intercepts CloudFront requests and rewrites the origin path
 * based on the version specified in S3 config/version.json
 */
exports.handler = async (event) => {
  const request = event.Records[0].cf.request;
  
  try {
    // Get current version from cache or S3
    const currentVersion = await getCurrentVersion();
    
    // Use fallback version if no version found from S3
    const versionToUse = currentVersion || FALLBACK_VERSION;
    
    if (!versionToUse) {
      console.error('No version available (neither from S3 nor fallback), proceeding with original request');
      return request;
    }
    
    // Rewrite the origin path to include the version
    const originalUri = request.uri;
    
    // Always strip any existing version prefix for security
    // This prevents clients from manipulating the version via URL
    const cleanUri = stripVersionPrefix(originalUri);
    
    // Handle root path requests
    if (cleanUri === '/' || cleanUri === '') {
      request.uri = `/${versionToUse}/index.html`;
    } else {
      // Prepend version to the clean path
      request.uri = `/${versionToUse}${cleanUri}`;
    }
    
    console.log(`Rewritten URI from ${originalUri} to ${request.uri} for version ${versionToUse}`);
    
  } catch (error) {
    console.error('Error in version selector:', error);
    // Use fallback version on error to maintain availability
    const fallbackUri = request.uri;
    const cleanUri = stripVersionPrefix(fallbackUri);
    
    if (FALLBACK_VERSION) {
      if (cleanUri === '/' || cleanUri === '') {
        request.uri = `/${FALLBACK_VERSION}/index.html`;
      } else {
        request.uri = `/${FALLBACK_VERSION}${cleanUri}`;
      }
      console.log(`Using fallback version ${FALLBACK_VERSION} due to error`);
    }
  }
  
  return request;
};

/**
 * Get current version with caching
 */
async function getCurrentVersion() {
  const now = Date.now();
  
  // Return cached version if still valid
  if (versionCache.version && (now - versionCache.timestamp) < versionCache.ttl) {
    console.log(`Using cached version: ${versionCache.version}`);
    return versionCache.version;
  }
  
  try {
    // Fetch new version from S3
    const s3 = new AWS.S3({ region: 'us-east-1' }); // Lambda@Edge requires us-east-1
    
    const params = {
      Bucket: BUCKET_NAME,
      Key: CONFIG_KEY
    };
    
    const result = await s3.getObject(params).promise();
    const configData = JSON.parse(result.Body.toString());
    
    if (configData.version) {
      // Update cache
      versionCache.version = configData.version;
      versionCache.timestamp = now;
      
      console.log(`Fetched and cached new version: ${configData.version}`);
      return configData.version;
    } else {
      console.error('Version not found in config file');
      return null;
    }
    
  } catch (error) {
    console.error('Error fetching version from S3:', error);
    
    // Return cached version if available, even if expired
    if (versionCache.version) {
      console.log(`Using stale cached version due to S3 error: ${versionCache.version}`);
      return versionCache.version;
    }
    
    return null;
  }
}

/**
 * Strip any existing version prefix from URI for security
 * This prevents clients from manipulating the version via URL
 */
function stripVersionPrefix(uri) {
  // Match pattern like /1.2.3/ or /v1.2.3/ at the beginning
  const versionPattern = /^\/v?\d+\.\d+\.\d+\//;
  
  if (versionPattern.test(uri)) {
    // Remove the version prefix, keeping the leading slash
    return uri.replace(versionPattern, '/');
  }
  
  return uri;
}

// Export for testing
exports.getCurrentVersion = getCurrentVersion;
exports.stripVersionPrefix = stripVersionPrefix;
exports.resetCache = () => {
  versionCache.version = null;
  versionCache.timestamp = 0;
};