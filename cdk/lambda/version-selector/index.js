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
    
    // Skip rewriting if no version found (fallback behavior)
    if (!currentVersion) {
      console.warn('No version found, proceeding with original request');
      return request;
    }
    
    // Rewrite the origin path to include the version
    const originalUri = request.uri;
    
    // Skip if already has the correct version prefix
    if (originalUri.startsWith(`/${currentVersion}/`)) {
      console.log(`URI already has correct version prefix: ${originalUri}`);
      return request;
    }
    
    // Handle root path requests
    if (originalUri === '/' || originalUri === '') {
      request.uri = `/${currentVersion}/index.html`;
    } else {
      // Prepend version to the path
      request.uri = `/${currentVersion}${originalUri}`;
    }
    
    console.log(`Rewritten URI from ${originalUri} to ${request.uri} for version ${currentVersion}`);
    
  } catch (error) {
    console.error('Error in version selector:', error);
    // Return original request on error to maintain availability
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

// Export for testing
exports.getCurrentVersion = getCurrentVersion;
exports.resetCache = () => {
  versionCache.version = null;
  versionCache.timestamp = 0;
};