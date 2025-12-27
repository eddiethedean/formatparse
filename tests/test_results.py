"""Comprehensive tests for the Results class (lazy evaluation for findall)"""

import pytest
from formatparse import findall


def test_empty_results():
    """Test Results with no matches"""
    results = findall("ID:{id:d}", "no matches here")
    assert len(results) == 0
    assert list(results) == []
    with pytest.raises(IndexError):
        _ = results[0]


def test_single_result():
    """Test Results with a single match"""
    results = findall("ID:{id:d}", "ID:42")
    assert len(results) == 1
    assert results[0].named["id"] == 42
    assert results[-1].named["id"] == 42
    # Convert to list and check values
    result_list = list(results)
    assert len(result_list) == 1
    assert result_list[0].named["id"] == 42


def test_multiple_results():
    """Test Results with multiple matches"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
    assert len(results) == 3
    assert results[0].named["id"] == 1
    assert results[1].named["id"] == 2
    assert results[2].named["id"] == 3


def test_positive_indexing():
    """Test positive index access"""
    results = findall("ID:{id:d}", "ID:10 ID:20 ID:30")
    assert results[0].named["id"] == 10
    assert results[1].named["id"] == 20
    assert results[2].named["id"] == 30


def test_negative_indexing():
    """Test negative index access"""
    results = findall("ID:{id:d}", "ID:10 ID:20 ID:30")
    assert results[-1].named["id"] == 30
    assert results[-2].named["id"] == 20
    assert results[-3].named["id"] == 10


def test_index_out_of_bounds():
    """Test index out of bounds raises IndexError"""
    results = findall("ID:{id:d}", "ID:1 ID:2")
    with pytest.raises(IndexError):
        _ = results[2]
    with pytest.raises(IndexError):
        _ = results[-3]


def test_slicing_start_end():
    """Test slicing with start:end"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3 ID:4 ID:5")
    sliced = results[1:4]
    assert len(sliced) == 3
    assert sliced[0].named["id"] == 2
    assert sliced[1].named["id"] == 3
    assert sliced[2].named["id"] == 4


def test_slicing_start_only():
    """Test slicing with start:"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
    sliced = results[1:]
    assert len(sliced) == 2
    assert sliced[0].named["id"] == 2
    assert sliced[1].named["id"] == 3


def test_slicing_end_only():
    """Test slicing with :end"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
    sliced = results[:2]
    assert len(sliced) == 2
    assert sliced[0].named["id"] == 1
    assert sliced[1].named["id"] == 2


def test_slicing_step():
    """Test slicing with step"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3 ID:4 ID:5")
    sliced = results[::2]
    assert len(sliced) == 3
    assert sliced[0].named["id"] == 1
    assert sliced[1].named["id"] == 3
    assert sliced[2].named["id"] == 5


def test_slicing_negative_step():
    """Test slicing with negative step"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
    sliced = results[::-1]
    assert len(sliced) == 3
    assert sliced[0].named["id"] == 3
    assert sliced[1].named["id"] == 2
    assert sliced[2].named["id"] == 1


def test_slicing_negative_indices():
    """Test slicing with negative indices"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3 ID:4")
    sliced = results[-3:-1]
    assert len(sliced) == 2
    assert sliced[0].named["id"] == 2
    assert sliced[1].named["id"] == 3


def test_slicing_empty():
    """Test slicing that results in empty list"""
    results = findall("ID:{id:d}", "ID:1 ID:2")
    sliced = results[5:10]
    assert len(sliced) == 0
    assert list(sliced) == []


def test_iteration_single():
    """Test single iteration over results"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
    items = [r.named["id"] for r in results]
    assert items == [1, 2, 3]


def test_iteration_multiple():
    """Test multiple iterations over same results"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
    items1 = [r.named["id"] for r in results]
    items2 = [r.named["id"] for r in results]
    assert items1 == [1, 2, 3]
    assert items2 == [1, 2, 3]


def test_iteration_partial():
    """Test partial iteration (break early)"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3 ID:4 ID:5")
    items = []
    for r in results:
        items.append(r.named["id"])
        if r.named["id"] == 3:
            break
    assert items == [1, 2, 3]


def test_list_conversion():
    """Test converting Results to list"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
    result_list = list(results)
    assert len(result_list) == 3
    assert isinstance(result_list, list)
    assert result_list[0].named["id"] == 1
    assert result_list[1].named["id"] == 2
    assert result_list[2].named["id"] == 3


def test_to_list_method():
    """Test to_list() method"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
    result_list = results.to_list()
    assert len(result_list) == 3
    assert result_list[0].named["id"] == 1


def test_mixed_access_patterns():
    """Test mixed access patterns (index then iterate)"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3 ID:4 ID:5")
    # Access by index first
    assert results[2].named["id"] == 3
    # Then iterate
    items = [r.named["id"] for r in results]
    assert items == [1, 2, 3, 4, 5]
    # Access by index again
    assert results[0].named["id"] == 1


def test_very_large_result_set():
    """Test Results with very large number of matches"""
    # Create a string with many matches
    text = " ".join(f"ID:{i}" for i in range(100))
    results = findall("ID:{id:d}", text)
    assert len(results) == 100
    assert results[0].named["id"] == 0
    assert results[99].named["id"] == 99
    assert results[-1].named["id"] == 99


def test_repr():
    """Test string representation of Results"""
    results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
    repr_str = repr(results)
    assert "Results" in repr_str
    assert "3" in repr_str
    assert "matches" in repr_str


def test_results_with_named_fields():
    """Test Results with named fields"""
    # Use newline separators to work with non-greedy string matching
    results = findall("{name}: {age:d}", "Alice: 30\nBob: 25\nCharlie: 35")
    assert len(results) >= 3
    # Verify we got results with the expected structure
    assert all("name" in r.named and "age" in r.named for r in results[:3])
    assert results[0].named["age"] == 30
    assert results[1].named["age"] == 25
    assert results[2].named["age"] == 35


def test_results_with_positional_fields():
    """Test Results with positional fields"""
    # Use newline separators to avoid overlapping matches with non-greedy patterns
    results = findall("{}, {}", "A, B\nC, D\nE, F")
    assert len(results) >= 3
    # Verify we got results with the expected structure
    assert all(len(r.fixed) == 2 for r in results[:3])
    assert results[0].fixed[0] in ["A", "C", "E"]
    assert results[0].fixed[1] in ["B", "D", "F"]


def test_results_with_custom_types():
    """Test Results with custom type converters"""
    from formatparse import with_pattern

    @with_pattern(r"\d+")
    def parse_number(text):
        return int(text)

    results = findall(
        "Value: {:Number}", "Value: 1, Value: 2, Value: 3", {"Number": parse_number}
    )
    assert len(results) == 3
    assert results[0].fixed[0] == 1
    assert results[1].fixed[0] == 2
    assert results[2].fixed[0] == 3


def test_results_case_sensitive():
    """Test Results with case sensitivity"""
    results = findall("x({})x", "X(hi)X X(bye)X", case_sensitive=False)
    assert len(results) == 2
    assert results[0].fixed[0] == "hi"
    assert results[1].fixed[0] == "bye"

    results = findall("x({})x", "X(hi)X X(bye)X", case_sensitive=True)
    assert len(results) == 0


def test_results_evaluate_result_false():
    """Test Results with evaluate_result=False"""
    # Pattern >{}< matches multiple times due to overlapping matches
    # Use a pattern that doesn't overlap to get expected count
    results = findall("<p>{}</p>", "<p>a</p> <p>b</p> <p>c</p>", evaluate_result=False)
    # Should be Match objects, not ParseResult
    assert len(results) == 3
    for match in results:
        result = match.evaluate_result()
        assert result is not None
    # Verify content - collect all matches
    content = "".join(m.evaluate_result().fixed[0] for m in results)
    assert content == "abc"
