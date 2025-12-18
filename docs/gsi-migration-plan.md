# DynamoDB GSI Migration Plan

## Background

DynamoDB has a limitation that prevents performing more than one Global Secondary Index (GSI) creation or deletion in a single CloudFormation update. This document outlines the strategy for managing GSI changes to the StreamWidgetsTable.

## Issue

The deployment failed with error: "Cannot perform more than one GSI creation or deletion in a single update" when attempting to modify the GSI structure of the StreamWidgetsTable.

**Root Cause**: The commit e6f2410 ("fix: update GSI for stream widgets to remove active sort key and adjust query logic") attempted to:
1. Remove an old GSI (likely `type-active-index` or `type-index` with a sort key on `active`)
2. Add a new GSI (`type-index` without a sort key)

This triggered DynamoDB's limitation since both operations cannot happen in a single update.

## Current State (as of this fix)

The StreamWidgetsTable now has the following GSI structure:

1. **user_id-index**: Query all widgets for a user (unchanged)
   - Partition Key: `user_id` (STRING)
   - Projection: ALL

2. **access_token-index**: WebSocket authentication (unchanged)
   - Partition Key: `access_token` (STRING)
   - Projection: ALL

3. **type-v2-index**: Query widgets by type for scheduled updates (NEW - temporary name)
   - Partition Key: `type` (STRING)
   - Projection: ALL
   - Note: Uses filter expression on `active` field since boolean cannot be used as sort key

## Deployed State (before this fix)

The deployed table likely had:
- `user_id-index` (same as current)
- `access_token-index` (same as current)
- Either `type-index` or `type-active-index` with a different structure (to be confirmed)

## Solution Implemented

Used a **temporary index name** (`type-v2-index`) to avoid conflicts:
- Adding a new GSI with a different name doesn't conflict with removing an old one
- The application code (`widget_updater_lambda`) was updated to use the new index name
- This allows the deployment to proceed with only one GSI operation (addition)

## Future Cleanup (Optional)

If the old GSI (`type-index` or `type-active-index`) still exists in the deployed table after this deployment succeeds, it can be removed in a follow-up deployment:

### Step 1: Verify Old GSI Still Exists
```bash
aws dynamodb describe-table --table-name <table-name> --query 'Table.GlobalSecondaryIndexes[*].IndexName'
```

### Step 2: Remove Old GSI (if it exists)
1. Remove the old GSI definition from `cdk/lib/datastore.ts`
2. Deploy the CDK stack
3. Wait for deployment to complete

### Step 3: Rename to Preferred Index Name (optional)
If you prefer `type-index` over `type-v2-index`:

**Deployment 1**: Add `type-index` alongside `type-v2-index`
```typescript
// Add both indexes temporarily
streamWidgetsTable.addGlobalSecondaryIndex({
  indexName: 'type-v2-index',
  // ... existing config
});

streamWidgetsTable.addGlobalSecondaryIndex({
  indexName: 'type-index',
  // ... same config
});
```

**Deployment 2**: Update application code to use `type-index`
- Update `widget_updater_lambda/src/main.rs` to use `type-index`
- Deploy the lambda

**Deployment 3**: Remove `type-v2-index`
- Remove the `type-v2-index` GSI definition
- Deploy the CDK stack

## Prevention for Future GSI Changes

When modifying GSI structure:

1. **Never remove and add GSIs in the same deployment**
2. **Stage changes across multiple deployments**:
   - Deployment 1: Add new GSI (or remove one GSI)
   - Deployment 2: Update application code to use new GSI (if adding)
   - Deployment 3: Remove old GSI (if adding)

3. **Use temporary names** to avoid conflicts during migration

4. **Verify the deployed state** before making changes:
   ```bash
   aws dynamodb describe-table --table-name <table-name>
   ```

## References

- AWS Documentation: [Managing Global Secondary Indexes](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/GSI.OnlineOps.html)
- DynamoDB Limitation: Only one GSI can be created or deleted per update operation
- Related Commit: e6f2410 ("fix: update GSI for stream widgets to remove active sort key and adjust query logic")

## Notes

- The `active` field is a boolean and cannot be used as a GSI sort key in DynamoDB
- The application uses a filter expression to query only active widgets
- This approach is acceptable for tables with relatively small result sets
- If performance becomes an issue, consider using a GSI with a computed string field that represents active status
