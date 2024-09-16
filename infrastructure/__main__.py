"""glowing-telegram: A tool for managing stream recordings"""

import pulumi
import pulumi_aws_native as aws_native

video_archive = aws_native.s3.Bucket(
    "video-archive",
    accelerate_configuration={
        "acceleration_status": aws_native.s3.BucketAccelerateConfigurationAccelerationStatus.ENABLED,
    },
    analytics_configurations=[
        {
            "id": "test",
            "storage_class_analysis": {},
        }
    ],
    bucket_encryption={
        "server_side_encryption_configuration": [
            {
                "bucket_key_enabled": True,
                "server_side_encryption_by_default": {
                    "sse_algorithm": aws_native.s3.BucketServerSideEncryptionByDefaultSseAlgorithm.AES256,
                },
            }
        ],
    },
    bucket_name="saebyn-video-archive",
    lifecycle_configuration={
        "rules": [
            {
                "abort_incomplete_multipart_upload": {
                    "days_after_initiation": 1,
                },
                "id": "delete_old_markers",
                "status": aws_native.s3.BucketRuleStatus.ENABLED,
            },
            {
                "id": "glacier archive",
                "status": aws_native.s3.BucketRuleStatus.ENABLED,
                "transitions": [
                    {
                        "storage_class": aws_native.s3.BucketTransitionStorageClass.STANDARD_IA,
                        "transition_in_days": 30,
                    },
                    {
                        "storage_class": aws_native.s3.BucketTransitionStorageClass.GLACIER_IR,
                        "transition_in_days": 60,
                    },
                    {
                        "storage_class": aws_native.s3.BucketTransitionStorageClass.GLACIER,
                        "transition_in_days": 150,
                    },
                ],
            },
        ],
    },
    ownership_controls={
        "rules": [
            {
                "object_ownership": aws_native.s3.BucketOwnershipControlsRuleObjectOwnership.BUCKET_OWNER_ENFORCED,
            }
        ],
    },
    public_access_block_configuration={
        "block_public_acls": True,
        "block_public_policy": True,
        "ignore_public_acls": True,
        "restrict_public_buckets": True,
    },
    versioning_configuration={
        "status": aws_native.s3.BucketVersioningConfigurationStatus.ENABLED,
    },
    opts=pulumi.ResourceOptions(protect=True),
)

ecr_repository = aws_native.ecr.Repository(
    "ecr-repository",
    image_scanning_configuration={
        "scan_on_push": True,
    },
)
