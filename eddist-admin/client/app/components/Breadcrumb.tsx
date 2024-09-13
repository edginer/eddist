import React from "react";

interface Props {
  children: React.ReactNode[];
}

const Breadcrumb = ({ children }: Props) => {
  return (
    <nav className="text-black font-bold my-2 ml-4" aria-label="Breadcrumb">
      <ol className="list-none p-0 inline-flex">
        {children.map((child, idx) => (
          <li key={idx} className="flex items-center">
            {child}
            {idx !== children.length - 1 && (
              <svg
                className="fill-current w-3 h-3 mx-3"
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 320 512"
              >
                <path d="M300.5 233.1L52.7 12.7C41.8 2.7 28.5-1.5 16.4.3-6.5 4.2-6.5 23.8 16.4 27.7l247.8 220.4c12.2 3.4 12.2 20.6 0 24.1L16.4 484.3c-22.9 4-22.9 29.7 0 33.7 12.1 1.8 25.4-1.1 36.3-11.1l247.8-220.4c12.2-3.4 12.2-20.6 0-24.1z" />
              </svg>
            )}
          </li>
        ))}
      </ol>
    </nav>
  );
};

export default Breadcrumb;
