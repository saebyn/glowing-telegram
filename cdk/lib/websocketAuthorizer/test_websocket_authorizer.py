"""
Tests for websocket authorizer main.py
"""
import pytest
import sys
import os
import importlib.util
from unittest.mock import patch, MagicMock


class TestWebSocketAuthorizer:
    """Tests for websocket authorizer"""
    
    @patch.dict(os.environ, {
        'USER_POOL_ID': 'test_pool_id',
        'USER_POOL_CLIENT_ID': 'test_client_id',
        'AWS_REGION': 'us-west-2'
    })
    def test_generate_policy(self):
        """Test the generate_policy function"""
        # Import the specific main.py file using importlib
        spec = importlib.util.spec_from_file_location(
            "websocket_auth_main", 
            os.path.join(os.path.dirname(__file__), "main.py")
        )
        auth_main = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(auth_main)
        
        policy = auth_main.generate_policy("user123", "Allow", "arn:aws:execute-api:*")
        
        assert policy["principalId"] == "user123"
        assert policy["policyDocument"]["Statement"][0]["Effect"] == "Allow"
        assert policy["policyDocument"]["Statement"][0]["Resource"] == "arn:aws:execute-api:*"
        
        # Test with context
        policy_with_context = auth_main.generate_policy("user123", "Allow", "arn:aws:execute-api:*", {"key": "value"})
        assert policy_with_context["context"]["key"] == "value"


class TestPythonFileExists:
    """Basic test to ensure Python file exists and has valid syntax"""
    
    def test_main_py_exists(self):
        """Test that main.py exists"""
        main_path = os.path.join(os.path.dirname(__file__), 'main.py')
        assert os.path.exists(main_path), "main.py does not exist"
    
    def test_main_py_syntax(self):
        """Test that main.py has valid syntax"""
        import ast
        
        main_path = os.path.join(os.path.dirname(__file__), 'main.py')
        if os.path.exists(main_path):
            with open(main_path, 'r') as f:
                try:
                    ast.parse(f.read())
                except SyntaxError:
                    pytest.fail("Syntax error in main.py")