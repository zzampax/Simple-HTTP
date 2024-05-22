function createCard(content) {
  let card = document.createElement("div");
  card.classList.add(
    "card",
    "w-full",
    "bg-base-100",
    "shadow-xl",
    "break-inside-avoid-column",
    "mb-4",
  );
  let owner = document.createElement("div");
  owner.classList.add("flex", "gap-4", "mx-8", "mt-8");
  let avatar = document.createElement("div");
  avatar.classList.add("avatar");
  let avatarImg = document.createElement("div");
  avatarImg.classList.add("w-12", "rounded-btn");
  let pfpOwner = document.createElement("img");
  pfpOwner.src =
    "https://img.daisyui.com/images/stock/photo-1534528741775-53994a69daeb.jpg";
  pfpOwner.alt = "Avatar";
  avatarImg.appendChild(pfpOwner);
  avatar.appendChild(avatarImg);
  let postDetails = document.createElement("div");
  postDetails.classList.add("flex", "flex-col");
  let ownerEmail = document.createElement("div");
  ownerEmail.classList.add("font-bold");
  ownerEmail.textContent = content.email;
  let postDate = document.createElement("div");
  postDate.classList.add("text-sm", "text-gray-500");
  postDate.textContent = content.datetime;
  postDetails.appendChild(ownerEmail);
  postDetails.appendChild(postDate);
  owner.appendChild(avatar);
  owner.appendChild(postDetails);

  let figure = document.createElement("figure");
  if (content.image) {
    figure.classList.add("px-10", "pt-10");
    let img = document.createElement("img");
    img.src = content.image;
    img.classList.add("rounded-xl");
    figure.appendChild(img);
  }

  let cardBody = document.createElement("div");
  cardBody.classList.add("card-body");
  let h2 = document.createElement("h2");
  h2.classList.add("card-title");
  h2.textContent = content.title;
  let p = document.createElement("p");
  p.classList.add("break-words", "whitespace-pre-line");
  p.textContent = decodeURIComponent(content.content);
  let divider = document.createElement("div");
  divider.classList.add("divider");
  divider.textContent = "Comments";
  let comments = document.createElement("div");
  comments.classList.add("w-full", "flex", "flex-col", "gap-4");
  content.comments.forEach((comment) => {
    let commentDiv = document.createElement("div");
    commentDiv.classList.add("flex", "gap-4");
    let avatar = document.createElement("div");
    avatar.classList.add("avatar");
    let avatarImg = document.createElement("div");
    avatarImg.classList.add("w-12", "rounded-btn");
    let pfp = document.createElement("img");
    pfp.src =
      "https://img.daisyui.com/images/stock/photo-1534528741775-53994a69daeb.jpg";
    pfp.alt = "Avatar";
    avatarImg.appendChild(pfp);
    avatar.appendChild(avatarImg);
    let commentContent = document.createElement("div");
    commentContent.classList.add("flex", "flex-col", "grow");
    let commentDetails = document.createElement("div");
    commentDetails.classList.add(
      "flex",
      "flex-col",
      "lg:flex-row",
      "lg:justify-between",
      "lg:items-center",
      "w-ful",
    );
    let commentOwner = document.createElement("div");
    commentOwner.classList.add("font-bold");
    commentOwner.textContent = comment.email;
    let commentDate = document.createElement("div");
    commentDate.classList.add("text-xs", "text-gray-500");
    commentDate.textContent = comment.datetime;
    commentDetails.appendChild(commentOwner);
    commentDetails.appendChild(commentDate);
    let commentText = document.createElement("div");
    commentText.classList.add("break-words", "whitespace-pre-line");
    commentText.textContent = decodeURIComponent(comment.content);
    commentContent.appendChild(commentDetails);
    commentContent.appendChild(commentText);
    commentDiv.appendChild(avatar);
    commentDiv.appendChild(commentContent);
    comments.appendChild(commentDiv);
  });

  if (content.comments.length === 0) {
    let noComments = document.createElement("div");
    noComments.classList.add("text-center", "text-gray-500");
    noComments.textContent = "No comments yet";
    comments.appendChild(noComments);
  }

  let form = document.createElement("form");
  form.setAttribute("action", "/api/comment");
  form.setAttribute("method", "POST");
  form.classList.add("card-actions", "justify-end", "mt-6");
  let hiddenInput = document.createElement("input");
  hiddenInput.type = "hidden";
  hiddenInput.setAttribute("name", "post_id");
  hiddenInput.value = content.post_id;
  let input = document.createElement("input");
  input.classList.add("input", "input-bordered", "grow");
  input.setAttribute("name", "content");
  input.placeholder = "Add a comment";
  let button = document.createElement("button");
  button.classList.add("btn", "btn-primary", "grow", "lg:grow-0");
  button.textContent = "Comment";
  form.appendChild(hiddenInput);
  form.appendChild(input);
  form.appendChild(button);

  card.appendChild(owner);
  if (content.image) {
    card.appendChild(figure);
  }
  cardBody.appendChild(h2);
  cardBody.appendChild(p);
  cardBody.appendChild(divider);
  cardBody.appendChild(comments);
  cardBody.appendChild(form);
  card.appendChild(cardBody);

  return card;
}

document.addEventListener("DOMContentLoaded", async () => {
  let response = await fetch("/api/posts");
  let posts = await response.json();
  let container = document.getElementById("posts");
  posts.forEach((post) => {
    let card = createCard(post);
    container.appendChild(card);
  });
});
