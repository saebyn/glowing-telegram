"""
This module defines a Pulumi component resource that represents a GPU Batch Job Queue and its associated resources, including the compute environment.
"""

import pulumi
import pulumi_aws_native as aws_native


class GPUBatchJobQueue(pulumi.ComponentResource):
    def __init__(self, name, vpc_id, subnet_ids, opts=None):
        super().__init__(
            "glowing_telegram:infrastructure:GPUBatchJobQueue", name, None, opts
        )

        # Create a service role for the compute environment
        compute_environment_service_role = aws_native.iam.Role(
            f"{name}-compute-environment-service-role",
            assume_role_policy_document={
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Principal": {
                            "Service": "batch.amazonaws.com",
                        },
                        "Action": "sts:AssumeRole",
                    },
                ],
            },
            managed_policy_arns=[
                "arn:aws:iam::aws:policy/service-role/AWSBatchServiceRole",
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create a role that allows the compute environment to manage spot instances
        spot_fleet_role = aws_native.iam.Role(
            f"{name}-spot-fleet-role",
            assume_role_policy_document={
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Principal": {
                            "Service": "spotfleet.amazonaws.com",
                        },
                        "Action": "sts:AssumeRole",
                    },
                ],
            },
            managed_policy_arns=[
                "arn:aws:iam::aws:policy/service-role/AmazonEC2SpotFleetTaggingRole",
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

        ecs_instance_role = aws_native.iam.Role(
            f"{name}-ecs-instance-role",
            assume_role_policy_document={
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Principal": {
                            "Service": "ec2.amazonaws.com",
                        },
                        "Action": "sts:AssumeRole",
                    },
                ],
            },
            managed_policy_arns=[
                "arn:aws:iam::aws:policy/service-role/AmazonEC2ContainerServiceforEC2Role",
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

        ecs_instance_profile = aws_native.iam.InstanceProfile(
            f"{name}-ecs-instance-profile",
            roles=[ecs_instance_role.role_name],
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create a security group for the compute environment
        compute_environment_security_group = aws_native.ec2.SecurityGroup(
            f"{name}-compute-environment-security-group",
            vpc_id=vpc_id,
            group_description="Security group for the compute environment",
            security_group_egress=[
                {
                    "cidr_ip": "0.0.0.0/0",
                    "from_port": 0,
                    "ip_protocol": "-1",
                    "to_port": 0,
                },
            ],
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create a compute environment for AWS Batch
        compute_environment = aws_native.batch.ComputeEnvironment(
            f"{name}-compute-environment",
            compute_resources={
                "allocationStrategy": "SPOT_PRICE_CAPACITY_OPTIMIZED",
                "minv_cpus": 0,
                "maxv_cpus": 16,
                "security_group_ids": [compute_environment_security_group.id],
                "subnets": subnet_ids,
                "type": "SPOT",
                "spotIamFleetRole": spot_fleet_role.arn,
                "instanceRole": ecs_instance_profile.arn,
                "instanceTypes": ["g4dn"],
            },
            type="MANAGED",
            service_role=compute_environment_service_role.arn,
            opts=pulumi.ResourceOptions(parent=self),
        )

        # Create an AWS batch queue
        job_queue = aws_native.batch.JobQueue(
            f"{name}-queue",
            compute_environment_order=[
                {
                    "compute_environment": compute_environment.compute_environment_arn,
                    "order": 1,
                },
            ],
            priority=1,
            opts=pulumi.ResourceOptions(
                parent=self,
            ),
        )

        self.job_queue_arn = job_queue.job_queue_arn

        self.register_outputs(
            {
                "job_queue": job_queue,
            }
        )
