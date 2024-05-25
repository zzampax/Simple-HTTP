function evalLinks(element) {
  let links = element.textContent.match(/https?:\/\/[^\s]+/g);
  if (links) {
    links.forEach((link) => {
      element.innerHTML = element.innerHTML.replace(
        link,
        `<a href="${link}" target="_blank" class="link text-primary">${link}</a>`,
      );
    });
  }
}

async function addReaction(reaction, id) {
  let response = await fetch("/api/reaction", {
    credentials: "same-origin",
    method: "POST",
    body: `reaction=${reaction}&post_id=${id}`,
  });
  let data = await response.json();
  if (data.status === "ok") {
    let reactions = document.getElementById(id).querySelector("#reactions");
    reactions.childNodes.forEach((reaction) => {
      if (reaction.classList.contains("btn-outline")) {
        let reactionCount = reaction.querySelector(".badge");
        reactionCount.textContent = parseInt(reactionCount.textContent) - 1;
        reaction.classList.remove("btn-outline");
      }
    });
    let reactionElement = reactions.querySelector(`.${reaction}`);
    reactionElement.classList.add("btn-outline");
    let reactionCount = reactionElement.querySelector(".badge");
    reactionCount.textContent = parseInt(reactionCount.textContent) + 1;
  }
}

async function getReaction(id) {
  let response = await fetch(`/api/userreaction?post_id=${id}`, {
    credentials: "same-origin",
    method: "GET",
  });
  let data = await response.json();
  return data.type;
}

async function createCard(content) {
  let card = document.createElement("div");
  card.id = content.post_id;
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
  pfpOwner.src = `https://api.dicebear.com/8.x/notionists-neutral/svg?seed=${content.email}`;
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
  evalLinks(p);

  let userReaction = await getReaction(content.post_id);

  let reactions = document.createElement("div");
  reactions.id = "reactions";
  reactions.classList.add("flex", "gap-4", "justify-end", "mt-4");
  let heart = document.createElement("div");
  heart.classList.add("btn", "btn-sm", "heart");
  if (userReaction === "heart") {
    heart.classList.add("btn-outline");
  }
  heart.textContent = "\u2764";
  let heartCount = document.createElement("div");
  heartCount.textContent = content.reactions.heart || 0;
  heartCount.classList.add("badge", "badge-secondary");
  heart.appendChild(heartCount);
  let thumbsUp = document.createElement("div");
  thumbsUp.classList.add("btn", "btn-sm", "thumbsUp");
  if (userReaction === "thumbsUp") {
    thumbsUp.classList.add("btn-outline");
  }
  thumbsUp.textContent = "\u{1F44D}";
  let thumbsUpCount = document.createElement("div");
  thumbsUpCount.textContent = content.reactions.thumbsUp || 0;
  thumbsUpCount.classList.add("badge", "badge-secondary");
  thumbsUp.appendChild(thumbsUpCount);
  let thumbsDown = document.createElement("div");
  thumbsDown.classList.add("btn", "btn-sm", "thumbsDown");
  if (userReaction === "thumbsDown") {
    thumbsDown.classList.add("btn-outline");
  }
  thumbsDown.textContent = "\u{1F44E}";
  let thumbsDownCount = document.createElement("div");
  thumbsDownCount.textContent = content.reactions.thumbsDown || 0;
  thumbsDownCount.classList.add("badge", "badge-secondary");
  thumbsDown.appendChild(thumbsDownCount);
  reactions.appendChild(heart);
  reactions.appendChild(thumbsUp);
  reactions.appendChild(thumbsDown);

  heart.addEventListener("click", () => {
    addReaction("heart", content.post_id);
  });
  thumbsUp.addEventListener("click", () => {
    addReaction("thumbsUp", content.post_id);
  });
  thumbsDown.addEventListener("click", () => {
    addReaction("thumbsDown", content.post_id);
  });

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
    pfp.src = `https://api.dicebear.com/8.x/notionists-neutral/svg?seed=${comment.email}`;
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
  input.setAttribute("required", "");
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
  cardBody.appendChild(reactions);
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
  posts.forEach(async (post) => {
    let card = await createCard(post);
    container.appendChild(card);
  });
});
