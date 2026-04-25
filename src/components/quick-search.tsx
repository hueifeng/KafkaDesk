import React, { useState } from 'react';
import { Link } from 'react-router-dom';

interface QuickSearchProps {
  routes: { path: string; label: string }[];
  recentItems: { path: string; label: string }[];
}

const QuickSearch: React.FC<QuickSearchProps> = ({ routes, recentItems }) => {
  const [searchValue, setSearchValue] = useState('');

  const filteredResults = [...recentItems, ...routes].filter((item) =>
    item.label.toLowerCase().includes(searchValue.toLowerCase())
  );

  return (
    <div className="quick-search">
      <input
        type="text"
        placeholder="Search..."
        value={searchValue}
        onChange={(e) => setSearchValue(e.target.value)}
        className="search-input field-shell"
      />
      <div className="search-results">
        {filteredResults.length > 0 ? (
          <ul>
            {filteredResults.map((result) => (
              <li key={result.path}>
                <Link to={result.path}>{result.label}</Link>
              </li>
            ))}
          </ul>
        ) : (
          <div className="no-results">No results found</div>
        )}
      </div>
    </div>
  );
};

export default QuickSearch;
