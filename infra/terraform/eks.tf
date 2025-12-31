# EKS Cluster Configuration

module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "~> 19.0"

  cluster_name    = "${local.name_prefix}-eks"
  cluster_version = var.eks_cluster_version

  vpc_id     = module.vpc.vpc_id
  subnet_ids = module.vpc.private_subnets

  # Cluster endpoint access
  cluster_endpoint_public_access  = true
  cluster_endpoint_private_access = true

  # Cluster encryption
  cluster_encryption_config = var.enable_eks_encryption ? [
    {
      provider_key_arn = aws_kms_key.eks[0].arn
      resources        = ["secrets"]
    }
  ] : []

  # Additional security groups
  cluster_security_group_additional_rules = {
    ingress_nodes_ephemeral_ports_tcp = {
      description                = "Nodes on ephemeral ports"
      protocol                   = "tcp"
      from_port                  = 1025
      to_port                    = 65535
      type                       = "ingress"
      source_node_security_group = true
    }
  }

  # EKS Managed Node Groups
  eks_managed_node_groups = {
    # General purpose node group
    general = {
      name           = "${local.name_prefix}-general"
      instance_types = var.eks_node_instance_types

      min_size     = var.eks_node_min_size
      max_size     = var.eks_node_max_size
      desired_size = var.eks_node_desired_size

      capacity_type = "ON_DEMAND"

      # Launch template configuration
      block_device_mappings = {
        xvda = {
          device_name = "/dev/xvda"
          ebs = {
            volume_size           = 100
            volume_type           = "gp3"
            iops                  = 3000
            throughput            = 150
            encrypted             = true
            delete_on_termination = true
          }
        }
      }

      # Metadata options
      metadata_options = {
        http_endpoint               = "enabled"
        http_tokens                 = "required"
        http_put_response_hop_limit = 1
      }

      # Taints and labels
      labels = {
        Environment = var.environment
        NodeGroup   = "general"
      }

      tags = merge(
        local.common_tags,
        {
          Name = "${local.name_prefix}-general-node"
        }
      )
    }

    # High-performance node group for trading strategies
    trading = {
      name           = "${local.name_prefix}-trading"
      instance_types = ["c6i.2xlarge", "c6i.4xlarge"]

      min_size     = 2
      max_size     = 8
      desired_size = 3

      capacity_type = "ON_DEMAND"

      block_device_mappings = {
        xvda = {
          device_name = "/dev/xvda"
          ebs = {
            volume_size           = 100
            volume_type           = "gp3"
            iops                  = 16000
            throughput            = 1000
            encrypted             = true
            delete_on_termination = true
          }
        }
      }

      labels = {
        Environment = var.environment
        NodeGroup   = "trading"
        Workload    = "low-latency"
      }

      taints = [
        {
          key    = "workload"
          value  = "trading"
          effect = "NoSchedule"
        }
      ]

      tags = merge(
        local.common_tags,
        {
          Name = "${local.name_prefix}-trading-node"
        }
      )
    }
  }

  # Cluster addons
  cluster_addons = {
    coredns = {
      most_recent = true
    }
    kube-proxy = {
      most_recent = true
    }
    vpc-cni = {
      most_recent = true
    }
    aws-ebs-csi-driver = {
      most_recent              = true
      service_account_role_arn = aws_iam_role.ebs_csi_driver.arn
    }
  }

  # OIDC provider for service accounts
  enable_irsa = true

  tags = local.common_tags
}

# KMS key for EKS encryption
resource "aws_kms_key" "eks" {
  count = var.enable_eks_encryption ? 1 : 0

  description             = "KMS key for EKS cluster encryption"
  deletion_window_in_days = 7
  enable_key_rotation     = true

  tags = merge(
    local.common_tags,
    {
      Name = "${local.name_prefix}-eks-kms"
    }
  )
}

resource "aws_kms_alias" "eks" {
  count = var.enable_eks_encryption ? 1 : 0

  name          = "alias/${local.name_prefix}-eks"
  target_key_id = aws_kms_key.eks[0].key_id
}

# IAM role for EBS CSI driver
resource "aws_iam_role" "ebs_csi_driver" {
  name = "${local.name_prefix}-ebs-csi-driver"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRoleWithWebIdentity"
        Effect = "Allow"
        Principal = {
          Federated = module.eks.oidc_provider_arn
        }
        Condition = {
          StringEquals = {
            "${module.eks.oidc_provider}:sub" = "system:serviceaccount:kube-system:ebs-csi-controller-sa"
            "${module.eks.oidc_provider}:aud" = "sts.amazonaws.com"
          }
        }
      }
    ]
  })

  tags = local.common_tags
}

resource "aws_iam_role_policy_attachment" "ebs_csi_driver" {
  role       = aws_iam_role.ebs_csi_driver.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonEBSCSIDriverPolicy"
}

# Kubernetes provider configuration
provider "kubernetes" {
  host                   = module.eks.cluster_endpoint
  cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

  exec {
    api_version = "client.authentication.k8s.io/v1beta1"
    command     = "aws"
    args = [
      "eks",
      "get-token",
      "--cluster-name",
      module.eks.cluster_name,
      "--region",
      var.aws_region
    ]
  }
}
