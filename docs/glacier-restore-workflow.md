# Glacier Restore Workflow

## Overview

The video ingestion Step Functions workflow now includes automatic Glacier restore functionality. This ensures that videos stored in Amazon S3 Glacier or Deep Archive storage classes are automatically restored before processing begins.

## Workflow Architecture

### Integration Point

The Glacier restore logic is integrated into the **Map state** that processes each video in parallel during stream ingestion. The restore check and wait logic occurs before the video metadata lookup and ingestion job submission.

### Workflow Steps

For each video in the stream:

```
┌─────────────────────────────────────┐
│ Check Object Storage Class          │
│ (S3 HeadObject API)                 │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│ Check if Restore Needed             │
│ (Choice State)                      │
└──────────────┬──────────────────────┘
               │
       ┌───────┴────────┐
       │                │
       ▼                ▼
    GLACIER/       STANDARD/IA/
    DEEP_ARCHIVE   other accessible
       │                │
       ▼                │
┌──────────────┐        │
│ Check Restore│        │
│ Status       │        │
└──────┬───────┘        │
       │                │
  ┌────┴─────┐          │
  │          │          │
  ▼          ▼          │
Already   Restore       │
Restored  In Progress   │
  │          │          │
  │     ┌────┴──────┐   │
  │     │ Wait 60s  │   │
  │     └────┬──────┘   │
  │          │          │
  │          └──────────┤
  │                     │
  ▼          Not Yet    │
Need to      Restored   │
Restore         │       │
  │             ▼       │
  │      ┌──────────┐   │
  │      │ Initiate │   │
  │      │ Restore  │   │
  │      │ (S3 API) │   │
  │      └────┬─────┘   │
  │           │         │
  │      ┌────▼─────┐   │
  │      │ Wait 60s │   │
  │      └────┬─────┘   │
  │           │         │
  │           └─────────┤
  │                     │
  └─────────────────────┤
                        │
                        ▼
              ┌─────────────────┐
              │ Get Video       │
              │ Metadata        │
              │ (DynamoDB)      │
              └─────┬───────────┘
                    │
                    ▼
              ┌─────────────────┐
              │ Check Ingestion │
              │ Version         │
              └─────┬───────────┘
                    │
                    ▼
              ┌─────────────────┐
              │ Ingest Video    │
              │ (Batch Job)     │
              └─────────────────┘
```

## Implementation Details

### Storage Class Detection

Uses S3 `HeadObject` API call to retrieve:
- `StorageClass`: The current storage tier
- `Restore`: The restore status (if applicable)

### Restore Decision Logic

**Requires Restore:**
- Storage class is `GLACIER`
- Storage class is `DEEP_ARCHIVE`

**Does NOT Require Restore:**
- `STANDARD`
- `STANDARD_IA`
- `INTELLIGENT_TIERING`
- `ONEZONE_IA`
- `GLACIER_IR` (Instant Retrieval)

### Restore Status Handling

1. **Object Not Yet Restored**: Initiates restore request
2. **Restore In Progress** (`ongoing-request="true"`): Waits and re-checks
3. **Object Already Restored** (`ongoing-request="false"`): Proceeds immediately

### Restore Configuration

- **Tier**: Standard (3-5 hours for Glacier, 12 hours for Deep Archive)
- **Days**: 1 day (restored object lifetime)
- **Polling Interval**: 60 seconds

### IAM Permissions

The Step Functions execution role requires:
```json
{
  "Action": [
    "s3:GetObject",
    "s3:RestoreObject"
  ],
  "Resource": "arn:aws:s3:::video-archive-bucket/*"
}
```

## Operational Characteristics

### Parallelism

The Map state processes multiple videos in parallel. Each video independently:
- Checks its own storage class
- Initiates restore if needed
- Waits for its own restore to complete

### Wait Time Considerations

**For Glacier Standard Tier:**
- Typical restore time: 3-5 hours
- Max wait time in workflow: Unlimited (60s polling until complete)
- Step Functions max execution: 1 year (Standard workflow)

**For Deep Archive:**
- Typical restore time: 12 hours
- Same polling behavior as Glacier

### Cost Optimization

- **Restore Tier**: Using Standard tier balances cost and speed
- **Restore Days**: 1 day minimizes storage costs while allowing processing
- **Bulk Tier Alternative**: Could be configured for cost savings if processing isn't time-sensitive

### Error Handling

Built-in Step Functions error handling:
- `States.TaskFailed`: S3 API call failures
- `States.Timeout`: If configured (none set currently)
- Automatic retry on transient failures

## Configuration

All configuration is in `cdk/lib/streamIngestion.ts`:

```typescript
// Polling interval
stepfunctions.WaitTime.duration(cdk.Duration.seconds(60))

// Restore parameters
RestoreRequest: {
  Days: 1,
  GlacierJobParameters: {
    Tier: 'Standard',
  },
}
```

## Monitoring

### CloudWatch Logs

Step Functions execution logs show:
- Storage class checks
- Restore initiation
- Wait states
- Decision points

### EventBridge Events

Stream ingestion status events include glacier restore timing:
- Overall execution time includes restore wait
- Individual step durations visible in execution history

### Metrics to Monitor

- Restore request frequency
- Average restore wait time
- Failed restore attempts
- Cost impact from restore requests

## Future Enhancements

Potential improvements:
1. **Configurable restore tier**: Allow Bulk tier for cost savings
2. **Configurable restore days**: Adjust based on processing window
3. **Pre-restore batching**: Initiate restores ahead of ingestion schedule
4. **Restore cost tracking**: Add custom metrics for restore costs
5. **Conditional tier selection**: Use Bulk for non-urgent, Standard for urgent
