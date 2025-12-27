"""Tests for fill character stripping from parsed results"""

import pytest
from formatparse import parse
from datetime import date


def test_right_aligned_fill_character():
    """Test right-aligned fields with fill character"""
    # Dot fill character
    result = parse('{name:.>16.16}', '.............Joe')
    assert result is not None
    assert result.named['name'] == 'Joe'
    
    # X fill character
    result = parse('{name:x>10.10}', 'xxxxxxxABC')
    assert result is not None
    assert result.named['name'] == 'ABC'
    
    # Zero fill character
    result = parse('{name:0>8.8}', '00000XYZ')
    assert result is not None
    assert result.named['name'] == 'XYZ'


def test_left_aligned_fill_character():
    """Test left-aligned fields with fill character"""
    # Dot fill character
    result = parse('{name:.<16.16}', 'Joe.............')
    assert result is not None
    assert result.named['name'] == 'Joe'
    
    # X fill character
    result = parse('{name:x<10.10}', 'ABCxxxxxxx')
    assert result is not None
    assert result.named['name'] == 'ABC'
    
    # Zero fill character
    result = parse('{name:0<8.8}', 'XYZ00000')
    assert result is not None
    assert result.named['name'] == 'XYZ'


def test_center_aligned_fill_character():
    """Test center-aligned fields with fill character"""
    # Dot fill character
    result = parse('{name:.^16.16}', '.....Joe......')
    assert result is not None
    assert result.named['name'] == 'Joe'
    
    # X fill character
    result = parse('{name:x^10.10}', 'xxxABCxxxx')
    assert result is not None
    assert result.named['name'] == 'ABC'
    
    # Zero fill character
    result = parse('{name:0^8.8}', '00XYZ000')
    assert result is not None
    assert result.named['name'] == 'XYZ'


def test_fill_character_with_whitespace():
    """Test fill character stripping with whitespace"""
    # Right-aligned with spaces and fill chars
    result = parse('{name:.>16.16}', '  .............Joe  ')
    assert result is not None
    assert result.named['name'] == 'Joe'
    
    # Left-aligned with spaces and fill chars
    result = parse('{name:.<16.16}', '  Joe.............  ')
    assert result is not None
    assert result.named['name'] == 'Joe'
    
    # Center-aligned with spaces and fill chars
    result = parse('{name:.^16.16}', '  .....Joe......  ')
    assert result is not None
    assert result.named['name'] == 'Joe'


def test_fill_character_in_content():
    """Test that fill characters in the middle of content are not stripped"""
    # Fill character appears in actual content
    result = parse('{name:.>10.10}', '....Joe.')
    assert result is not None
    # Should strip leading dots but keep trailing dot that's part of content
    assert result.named['name'] == 'Joe.'
    
    # Multiple fill characters in content
    result = parse('{name:x>10.10}', 'xxxJoexxx')
    assert result is not None
    # Should strip leading x's but keep x's that are part of content
    assert result.named['name'] == 'Joexxx'


def test_no_fill_character_still_strips_whitespace():
    """Test that alignment without fill character still strips whitespace"""
    # Right-aligned without fill
    result = parse('{name:>10}', '      Joe')
    assert result is not None
    assert result.named['name'] == 'Joe'
    
    # Left-aligned without fill
    result = parse('{name:<10}', 'Joe      ')
    assert result is not None
    assert result.named['name'] == 'Joe'
    
    # Center-aligned without fill
    result = parse('{name:^10}', '   Joe   ')
    assert result is not None
    assert result.named['name'] == 'Joe'


def test_round_trip_fill_character():
    """Test round-trip compatibility with format strings"""
    from datetime import date
    
    # Test with date and name
    pattern = 'Date: {date:%Y%m%d} Name: {name:.>16.16}'
    formatted = pattern.format(date=date(2025, 10, 31), name='Joe')
    result = parse(pattern, formatted)
    assert result is not None
    assert result.named['name'] == 'Joe'  # Should be 'Joe', not '.............Joe'
    assert result.named['date'] == date(2025, 10, 31)


def test_round_trip_different_fill_characters():
    """Test round-trip with different fill characters"""
    # Zero fill
    pattern = 'ID: {id:0>8.8}'
    formatted = pattern.format(id='123')
    result = parse(pattern, formatted)
    assert result is not None
    assert result.named['id'] == '123'
    
    # X fill
    pattern = 'Code: {code:x>10.10}'
    formatted = pattern.format(code='ABC')
    result = parse(pattern, formatted)
    assert result is not None
    assert result.named['code'] == 'ABC'
    
    # Dash fill
    pattern = 'Value: {value:-<12.12}'
    formatted = pattern.format(value='test')
    result = parse(pattern, formatted)
    assert result is not None
    assert result.named['value'] == 'test'


def test_all_alignment_types_with_fill():
    """Test all alignment types comprehensively"""
    test_cases = [
        ('{name:.>10.10}', '.......ABC', '>', 'ABC'),
        ('{name:.<10.10}', 'ABC.......', '<', 'ABC'),
        ('{name:.^10.10}', '...ABC....', '^', 'ABC'),
        ('{name:x>8.8}', 'xxxxxXYZ', '>', 'XYZ'),
        ('{name:x<8.8}', 'XYZxxxxx', '<', 'XYZ'),
        ('{name:x^8.8}', 'xxXYZxxx', '^', 'XYZ'),
    ]
    
    for pattern, input_str, align, expected in test_cases:
        result = parse(pattern, input_str)
        assert result is not None, f"Failed to parse {pattern} with {input_str}"
        assert result.named['name'] == expected, \
            f"Alignment {align}: expected '{expected}', got '{result.named['name']}'"


def test_empty_string_with_fill():
    """Test edge case with empty string and fill characters"""
    # Right-aligned empty
    result = parse('{name:.>10.10}', '..........')
    assert result is not None
    assert result.named['name'] == ''
    
    # Left-aligned empty
    result = parse('{name:.<10.10}', '..........')
    assert result is not None
    assert result.named['name'] == ''
    
    # Center-aligned empty
    result = parse('{name:.^10.10}', '..........')
    assert result is not None
    assert result.named['name'] == ''


def test_no_alignment_no_stripping():
    """Test that without alignment, nothing is stripped"""
    # No alignment, no fill
    result = parse('{name}', 'Joe')
    assert result is not None
    assert result.named['name'] == 'Joe'
    
    # No alignment, even with fill in pattern (shouldn't matter)
    result = parse('{name:.16}', 'Joe')
    assert result is not None
    assert result.named['name'] == 'Joe'


def test_fill_character_with_precision():
    """Test fill character with precision specification"""
    # Right-aligned with precision
    result = parse('{name:.>10.5}', '.....Hello')
    assert result is not None
    assert result.named['name'] == 'Hello'
    
    # Left-aligned with precision
    result = parse('{name:.<10.5}', 'Hello.....')
    assert result is not None
    assert result.named['name'] == 'Hello'
    
    # Center-aligned with precision
    result = parse('{name:.^10.5}', '..Hello...')
    assert result is not None
    assert result.named['name'] == 'Hello'

